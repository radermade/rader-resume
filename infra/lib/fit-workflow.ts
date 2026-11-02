import { Duration, RemovalPolicy } from 'aws-cdk-lib';
import { Alarm, ComparisonOperator, TreatMissingData } from 'aws-cdk-lib/aws-cloudwatch';
import { Architecture } from 'aws-cdk-lib/aws-lambda';
import { LogGroup, RetentionDays } from 'aws-cdk-lib/aws-logs';
import {
  DefinitionBody,
  JitterType,
  JsonPath,
  LogLevel,
  Parallel,
  Pass,
  Result,
  RetryProps,
  StateMachine,
  StateMachineType,
  TaskInput,
} from 'aws-cdk-lib/aws-stepfunctions';
import { LambdaInvoke } from 'aws-cdk-lib/aws-stepfunctions-tasks';
import { RustFunction } from 'cargo-lambda-cdk';
import { Construct } from 'constructs';

export interface FitWorkflowProps {
  readonly stage: string;
}

/**
 * The job description fit analysis, as an Express workflow.
 *
 * Express rather than Standard, invoked synchronously, because the analysis is
 * deterministic and completes in milliseconds. Standard plus polling would
 * need a projection, a store, a poll endpoint, and a TTL to absorb latency
 * that does not exist.
 *
 * The orchestration is not here for throughput. Four deterministic matchers
 * over a few dozen strings cost less than the state transitions that
 * coordinate them. It is here because each branch is a genuinely different
 * matcher with a different rule, per-branch failure is isolated, the execution
 * log gives per-branch timing, and a branch that later needs I/O does not
 * force a rewrite.
 */
/**
 * Transient faults in the invocation itself, not in the task's logic.
 *
 * A deterministic matcher that fails does so for the same reason every time,
 * so retrying it is pointless. These errors are the ones where a second
 * attempt is genuinely different.
 */
const TRANSIENT_INVOKE_ERRORS = [
  'Lambda.ServiceException',
  'Lambda.AWSLambdaException',
  'Lambda.SdkClientException',
  'Lambda.TooManyRequestsException',
];

/** Full jitter, because four branches retrying in lockstep is the bad case. */
const RETRY: RetryProps = {
  errors: TRANSIENT_INVOKE_ERRORS,
  interval: Duration.seconds(1),
  backoffRate: 2,
  maxAttempts: 3,
  jitterStrategy: JitterType.FULL,
};

export class FitWorkflow extends Construct {
  readonly stateMachine: StateMachine;
  /** Wired to an action by the stack, which owns where alerts go. */
  readonly failedExecutions: Alarm;

  constructor(scope: Construct, id: string, props: FitWorkflowProps) {
    super(scope, id);

    const logGroup = new LogGroup(this, 'TaskLogs', {
      retention: props.stage === 'prod' ? RetentionDays.ONE_MONTH : RetentionDays.TWO_WEEKS,
      removalPolicy: RemovalPolicy.DESTROY,
    });

    // One function for all six states. They are pure compute with no AWS
    // permissions and identical resource needs, so separate artifacts would be
    // copies without benefit. Isolation comes from separate invocations.
    const task = new RustFunction(this, 'FitTask', {
      manifestPath: '../crates/fit-task/Cargo.toml',
      architecture: Architecture.ARM_64,
      memorySize: 256,
      timeout: Duration.seconds(10),
      logGroup,
    });

    const extract = new LambdaInvoke(this, 'Extract', {
      lambdaFunction: task,
      // One explicit Retry, not the CDK default plus ours. The default is
      // emitted first and shadows anything overlapping it, and it carries no
      // jitter.
      retryOnServiceExceptions: false,
      payload: TaskInput.fromObject({
        step: 'extract',
        description: JsonPath.stringAt('$.description'),
      }),
      // Keep only what later states need. The description itself is not
      // carried forward: nothing downstream reads it, and dropping it keeps
      // visitor text out of the execution log.
      resultSelector: {
        requirements: JsonPath.objectAt('$.Payload.requirements'),
        fingerprint: JsonPath.stringAt('$.Payload.fingerprint'),
        // Carried because the description is dropped here. The count of
        // requirements the taxonomy could not place cannot be recovered from
        // the requirements alone, and a report that cannot state its own
        // coverage silently claims full coverage.
        unmatchedTerms: JsonPath.numberAt('$.Payload.unmatchedTerms'),
      },
      resultPath: '$.extracted',
    }).addRetry(RETRY);

    const branch = (dimension: string) => {
      const name = `${dimension[0].toUpperCase()}${dimension.slice(1)}`;

      // The shape a branch returns, with nothing in it. `aggregate` derives
      // the incomplete dimension by comparing requirements against
      // assessments, so an empty branch produces a report that names what is
      // missing rather than one that quietly omits it.
      //
      // This is R6.22: one broken matcher costs that dimension, not the report.
      const unavailable = new Pass(this, `Match${name}Unavailable`, {
        result: Result.fromObject({ dimension, assessments: [] }),
      });

      return new LambdaInvoke(this, `Match${name}`, {
        lambdaFunction: task,
        // One explicit Retry, not the CDK default plus ours. The default is
        // emitted first and shadows anything overlapping it, and it carries
        // no jitter.
        retryOnServiceExceptions: false,
        payload: TaskInput.fromObject({
          step: 'match',
          dimension,
          requirements: JsonPath.objectAt('$.extracted.requirements'),
          asOf: JsonPath.stringAt('$.asOf'),
        }),
        outputPath: '$.Payload',
      })
        .addRetry(RETRY)
        .addCatch(unavailable, { resultPath: JsonPath.DISCARD });
    };

    const match = new Parallel(this, 'MatchDimensions', {
      resultPath: '$.branches',
    })
      .branch(branch('tech'))
      .branch(branch('skills'))
      .branch(branch('experience'))
      .branch(branch('seniority'));

    const aggregate = new LambdaInvoke(this, 'Aggregate', {
      lambdaFunction: task,
      // One explicit Retry, not the CDK default plus ours. The default is
      // emitted first and shadows anything overlapping it, and it carries no
      // jitter.
      retryOnServiceExceptions: false,
      payload: TaskInput.fromObject({
        step: 'aggregate',
        requirements: JsonPath.objectAt('$.extracted.requirements'),
        branches: JsonPath.objectAt('$.branches'),
        generatedAt: JsonPath.stringAt('$.generatedAt'),
        unmatchedTerms: JsonPath.numberAt('$.extracted.unmatchedTerms'),
      }),
      outputPath: '$.Payload',
    }).addRetry(RETRY);

    const executionLogs = new LogGroup(this, 'ExecutionLogs', {
      retention: props.stage === 'prod' ? RetentionDays.ONE_MONTH : RetentionDays.TWO_WEEKS,
      removalPolicy: RemovalPolicy.DESTROY,
    });

    this.stateMachine = new StateMachine(this, 'FitStateMachine', {
      stateMachineType: StateMachineType.EXPRESS,
      definitionBody: DefinitionBody.fromChainable(extract.next(match).next(aggregate)),
      timeout: Duration.seconds(30),
      // Express workflows keep no execution history of their own, so without
      // this the per-branch timing that justifies the workflow is invisible.
      logs: {
        destination: executionLogs,
        level: props.stage === 'prod' ? LogLevel.ERROR : LogLevel.ALL,
        // Visitor-submitted text reaches the Extract input. Execution data is
        // logged in dev for debugging and withheld in prod.
        includeExecutionData: props.stage !== 'prod',
      },
      tracingEnabled: true,
    });

    // Extract and Aggregate deliberately have no Catch, which departs from the
    // "every Task state has a Catch" rule and is worth the note.
    //
    // A Catch exists to produce a degraded but useful outcome. There is no
    // useful partial outcome here: without extraction there is nothing to
    // match, and without aggregation there is no report. The handler already
    // falls back to running identical logic inline, which yields a complete
    // report rather than a degraded one, so letting the execution fail is the
    // better outcome. The branches are different, and they do have Catches.
    //
    // That fallback is also what makes this alarm mandatory rather than nice
    // to have: a visitor still gets a correct answer when the workflow is
    // broken, so nothing else would ever reveal that it is broken.
    this.failedExecutions = new Alarm(this, 'FailedExecutions', {
      metric: this.stateMachine.metricFailed({ period: Duration.minutes(5) }),
      threshold: 1,
      evaluationPeriods: 1,
      comparisonOperator: ComparisonOperator.GREATER_THAN_OR_EQUAL_TO_THRESHOLD,
      // No executions is the normal state on a site this quiet. Treating
      // missing data as breaching would alarm constantly.
      treatMissingData: TreatMissingData.NOT_BREACHING,
      alarmDescription:
        `Fit analysis workflow failed (${props.stage}). The handler falls back ` +
        'to inline analysis, so visitors are unaffected and this is the only ' +
        'signal that the workflow is broken.',
    });
  }
}
