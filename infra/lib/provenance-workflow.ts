import { Duration, RemovalPolicy } from 'aws-cdk-lib';
import { Alarm, ComparisonOperator, TreatMissingData } from 'aws-cdk-lib/aws-cloudwatch';
import { AttributeType, BillingMode, Table, TableEncryption } from 'aws-cdk-lib/aws-dynamodb';
import { Architecture } from 'aws-cdk-lib/aws-lambda';
import { LogGroup, RetentionDays } from 'aws-cdk-lib/aws-logs';
import {
  DefinitionBody,
  JitterType,
  JsonPath,
  LogLevel,
  Map as SfnMap,
  RetryProps,
  StateMachine,
  StateMachineType,
  TaskInput,
} from 'aws-cdk-lib/aws-stepfunctions';
import { LambdaInvoke } from 'aws-cdk-lib/aws-stepfunctions-tasks';
import { RustFunction } from 'cargo-lambda-cdk';
import { Construct } from 'constructs';

export interface ProvenanceWorkflowProps {
  readonly stage: string;
  /** Origin the checks resolve this site's own artifacts against. */
  readonly siteUrl: string;
}

/**
 * The live provenance check, as a Standard workflow.
 *
 * Standard rather than Express, and asynchronous rather than synchronous,
 * because a dozen outbound calls with timeouts will not finish inside a
 * request. Each target fails independently for reasons outside this system, so
 * per-branch retry and a tolerated failure percentage are doing real work.
 *
 * The opposite pattern to the fit workflow on purpose: that one is Express,
 * `Parallel`, and synchronous. Demonstrating one pattern twice would be worth
 * less than demonstrating two.
 */
/**
 * Transient faults in the invocation, not in the check.
 *
 * Worth being precise about what this does and does not fix. A target that
 * times out or blackholes does not fail the task: `check-runner` catches it and
 * returns `Unreachable`, which is a result. So this retry never fires for an
 * unreachable target, and a transient network blip is still reported as
 * unreachable. Fixing that means retrying inside `check-runner`, before it
 * concludes anything, which is a different change in a different crate.
 *
 * What this does cover is the invocation failing: throttling, a cold-start
 * fault, the service erroring. Those are worth a second attempt.
 */
const TRANSIENT_INVOKE_ERRORS = [
  'Lambda.ServiceException',
  'Lambda.AWSLambdaException',
  'Lambda.SdkClientException',
  'Lambda.TooManyRequestsException',
];

/**
 * Full jitter, and deliberately slower than the fit workflow's.
 *
 * Retries here can turn into outbound requests at third parties, so spreading
 * them out matters more than finishing quickly.
 */
const RETRY: RetryProps = {
  errors: TRANSIENT_INVOKE_ERRORS,
  interval: Duration.seconds(2),
  backoffRate: 2,
  maxAttempts: 3,
  jitterStrategy: JitterType.FULL,
};

export class ProvenanceWorkflow extends Construct {
  readonly stateMachine: StateMachine;
  readonly table: Table;
  /** Wired to an action by the stack, which owns where alerts go. */
  readonly failedExecutions: Alarm;

  constructor(scope: Construct, id: string, props: ProvenanceWorkflowProps) {
    super(scope, id);

    // A run is disposable: losing one costs a visitor a re-click. On-demand
    // billing because traffic is a handful of clicks a day, and a TTL so the
    // table never grows.
    this.table = new Table(this, 'Runs', {
      partitionKey: { name: 'runId', type: AttributeType.STRING },
      billingMode: BillingMode.PAY_PER_REQUEST,
      timeToLiveAttribute: 'expiresAt',
      encryption: TableEncryption.AWS_MANAGED,
      removalPolicy: RemovalPolicy.DESTROY,
    });

    const logGroup = new LogGroup(this, 'TaskLogs', {
      retention: props.stage === 'prod' ? RetentionDays.ONE_MONTH : RetentionDays.TWO_WEEKS,
      removalPolicy: RemovalPolicy.DESTROY,
    });

    const task = new RustFunction(this, 'ProvenanceTask', {
      manifestPath: '../crates/provenance-task/Cargo.toml',
      architecture: Architecture.ARM_64,
      memorySize: 256,
      // Longer than the per-request timeout in check-runner, so a slow target
      // is the checker's timeout rather than the Lambda's.
      timeout: Duration.seconds(20),
      logGroup,
      environment: {
        PROVENANCE_TABLE: this.table.tableName,
        SITE_URL: props.siteUrl,
      },
    });

    this.table.grantReadWriteData(task);

    const begin = new LambdaInvoke(this, 'Begin', {
      lambdaFunction: task,
      // One explicit Retry, not the CDK default plus ours. The default is
      // emitted first and shadows anything overlapping it, and it carries no
      // jitter.
      retryOnServiceExceptions: false,
      payload: TaskInput.fromObject({
        step: 'begin',
        runId: JsonPath.stringAt('$.runId'),
      }),
      resultPath: JsonPath.DISCARD,
    }).addRetry(RETRY);

    const runCheck = new LambdaInvoke(this, 'RunCheck', {
      lambdaFunction: task,
      // One explicit Retry, not the CDK default plus ours. The default is
      // emitted first and shadows anything overlapping it, and it carries no
      // jitter.
      retryOnServiceExceptions: false,
      payload: TaskInput.fromObject({
        step: 'runCheck',
        runId: JsonPath.stringAt('$.runId'),
        check: JsonPath.objectAt('$.check'),
      }),
      resultPath: JsonPath.DISCARD,
    }).addRetry(RETRY);

    const checks = new SfnMap(this, 'RunChecks', {
      itemsPath: '$.checks',
      // Bounded on purpose: this fans out at third parties who did not opt in,
      // and a trickle is neighbourly where a burst is not.
      maxConcurrency: 4,
      // `itemSelector` rather than the deprecated `parameters`. The item comes
      // from the context object, hence `$$`; a single `$` would resolve
      // against the state input, which has no `Map` key.
      itemSelector: {
        runId: JsonPath.stringAt('$.runId'),
        check: JsonPath.objectAt('$$.Map.Item.Value'),
      },
      resultPath: JsonPath.DISCARD,
    });

    checks.itemProcessor(runCheck);

    const finish = new LambdaInvoke(this, 'Finish', {
      lambdaFunction: task,
      // One explicit Retry, not the CDK default plus ours. The default is
      // emitted first and shadows anything overlapping it, and it carries no
      // jitter.
      retryOnServiceExceptions: false,
      payload: TaskInput.fromObject({
        step: 'finish',
        runId: JsonPath.stringAt('$.runId'),
      }),
      resultPath: JsonPath.DISCARD,
    }).addRetry(RETRY);

    const recordFailure = new LambdaInvoke(this, 'RecordFailure', {
      lambdaFunction: task,
      // One explicit Retry, not the CDK default plus ours. The default is
      // emitted first and shadows anything overlapping it, and it carries no
      // jitter.
      retryOnServiceExceptions: false,
      payload: TaskInput.fromObject({
        step: 'fail',
        runId: JsonPath.stringAt('$.runId'),
        // A fixed string rather than the raw error. This field is rendered to
        // a visitor and should not carry internal detail.
        diagnostic: 'The check could not be completed.',
      }),
      resultPath: JsonPath.DISCARD,
      // Retried harder than the rest. If this write is lost the run sits at
      // "running" until its TTL expires, and the visitor watches a progress
      // bar that will never move.
    }).addRetry({ ...RETRY, maxAttempts: 5 });

    // Every check is allowed to fail without failing the run. A target that
    // cannot be reached is a result, and "eleven of twelve passed, one timed
    // out" is both the honest answer and the likely one.
    checks.addCatch(recordFailure, { resultPath: JsonPath.DISCARD });
    begin.addCatch(recordFailure, { resultPath: JsonPath.DISCARD });

    const executionLogs = new LogGroup(this, 'ExecutionLogs', {
      retention: props.stage === 'prod' ? RetentionDays.ONE_MONTH : RetentionDays.TWO_WEEKS,
      removalPolicy: RemovalPolicy.DESTROY,
    });

    this.stateMachine = new StateMachine(this, 'ProvenanceStateMachine', {
      stateMachineType: StateMachineType.STANDARD,
      definitionBody: DefinitionBody.fromChainable(begin.next(checks).next(finish)),
      // Generous relative to the work, so a run of slow targets completes
      // rather than being cut off mid-fan-out.
      timeout: Duration.minutes(5),
      logs: {
        destination: executionLogs,
        level: props.stage === 'prod' ? LogLevel.ERROR : LogLevel.ALL,
        // No visitor-submitted content reaches this workflow, unlike the fit
        // one, so execution data is safe to log in both stages.
        includeExecutionData: true,
      },
      tracingEnabled: true,
    });

    // Only fires when the run itself broke, not when a check failed. A check
    // that cannot reach its target is a result and is reported as one, so this
    // alarm stays quiet during exactly the situation it would be tempting to
    // alert on.
    this.failedExecutions = new Alarm(this, 'FailedExecutions', {
      metric: this.stateMachine.metricFailed({ period: Duration.minutes(5) }),
      threshold: 1,
      evaluationPeriods: 1,
      comparisonOperator: ComparisonOperator.GREATER_THAN_OR_EQUAL_TO_THRESHOLD,
      treatMissingData: TreatMissingData.NOT_BREACHING,
      alarmDescription:
        `Provenance check workflow failed (${props.stage}). Individual ` +
        'unreachable targets do not trigger this; the run breaking does.',
    });
  }
}
