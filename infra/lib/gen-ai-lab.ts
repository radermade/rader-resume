import { Duration, RemovalPolicy } from 'aws-cdk-lib';
import { Alarm, ComparisonOperator, TreatMissingData } from 'aws-cdk-lib/aws-cloudwatch';
import { Architecture, FunctionUrl, FunctionUrlAuthType } from 'aws-cdk-lib/aws-lambda';
import { LogGroup, RetentionDays } from 'aws-cdk-lib/aws-logs';
import {
  BlockPublicAccess,
  Bucket,
  BucketEncryption,
  ObjectOwnership,
} from 'aws-cdk-lib/aws-s3';
import { BucketDeployment, CacheControl, Source } from 'aws-cdk-lib/aws-s3-deployment';
import { existsSync } from 'node:fs';
import { RustFunction } from 'cargo-lambda-cdk';
import { Construct } from 'constructs';

export interface GenAiLabProps {
  readonly stage: string;
}

/**
 * The GenAI lab: a second binary on the same distribution.
 *
 * Deliberately small. No DynamoDB, no model call, no state machine, and no
 * session: a scenario is content compiled into the binary, so serving a step
 * is rendering markup from embedded data. The step a visitor is on lives in a
 * posted form and nowhere else.
 *
 * Separate from the resume's function rather than new routes on it, so the
 * resume's palette, policy, and payload are untouched and either can be
 * deployed without rebuilding the other.
 */
export class GenAiLab extends Construct {
  readonly url: FunctionUrl;
  /**
   * The large, binary runtime assets: Pyodide core and the Python wheels.
   *
   * These do not fit the Lambda's embedded-string asset table — they are
   * multi-megabyte and binary, and embedding them would bloat the binary and
   * slow cold start. They live here, content-addressed and immutable, and the
   * stack fronts this bucket with a CloudFront behavior for
   * `/gen-ai/assets/pyodide/*`. The small hand-written scripts stay embedded
   * in the Lambda; only the runtime moved out. See
   * `.kiro/aws-gen-ai-app-roadmap/phase2/self-hosting.md`.
   */
  readonly assetBucket: Bucket;
  /** Wired to an action by the stack, which owns where alerts go. */
  readonly errors: Alarm;
  readonly throttles: Alarm;

  constructor(scope: Construct, id: string, props: GenAiLabProps) {
    super(scope, id);

    // Retention set explicitly. The Lambda default never expires and becomes
    // the largest line item on a project this size.
    const logGroup = new LogGroup(this, 'Logs', {
      retention: props.stage === 'prod' ? RetentionDays.ONE_MONTH : RetentionDays.TWO_WEEKS,
      removalPolicy: RemovalPolicy.DESTROY,
    });

    const handler = new RustFunction(this, 'Handler', {
      manifestPath: '../crates/gen-ai-handler/Cargo.toml',
      architecture: Architecture.ARM_64,
      // Rendering embedded markup. No model latency to absorb and nothing to
      // buffer, so the smallest tier and a short ceiling are honest rather
      // than optimistic.
      memorySize: 128,
      timeout: Duration.seconds(3),
      logGroup,
      environment: {
        STAGE: props.stage,
      },
    });

    // Public read path behind CloudFront, the same shape the resume uses.
    // Nothing here writes, so there is no mutating endpoint to authenticate:
    // the POST that advances a step returns a rendered page and stores
    // nothing.
    this.url = handler.addFunctionUrl({
      authType: FunctionUrlAuthType.NONE,
    });

    // The runtime asset bucket. Private: CloudFront reaches it with OAC, so
    // public access stays blocked and nothing is readable except through the
    // distribution. Versioned and SSE per the storage standards; the stack
    // adds the OAC grant and the behavior.
    //
    // Objects are content-addressed under `pyodide/`, so a long immutable
    // cache never serves stale: a version bump is a new key. The bucket is
    // retained on stack destroy in prod, because re-vendoring the runtime is
    // manual; dev is disposable.
    this.assetBucket = new Bucket(this, 'Assets', {
      blockPublicAccess: BlockPublicAccess.BLOCK_ALL,
      encryption: BucketEncryption.S3_MANAGED,
      enforceSSL: true,
      versioned: true,
      objectOwnership: ObjectOwnership.BUCKET_OWNER_ENFORCED,
      removalPolicy: props.stage === 'prod' ? RemovalPolicy.RETAIN : RemovalPolicy.DESTROY,
      autoDeleteObjects: props.stage !== 'prod',
      lifecycleRules: [
        {
          // Keep the current and prior generations; expire older noncurrent
          // versions so a re-vendor does not accumulate cost.
          noncurrentVersionExpiration: Duration.days(30),
          abortIncompleteMultipartUploadAfter: Duration.days(7),
        },
      ],
    });

    // Upload the vendored runtime, if it has been vendored. The files are
    // large and fetched by a manual step (see self-hosting.md), so the deploy
    // must not require them to exist: an empty or absent directory means the
    // bucket deploys empty and the workbench hangs honestly until the runtime
    // is present, rather than failing the whole stack.
    const vendoredRuntime = '../assets/gen-ai/pyodide';
    if (existsSync(vendoredRuntime)) {
      new BucketDeployment(this, 'RuntimeDeployment', {
        sources: [Source.asset(vendoredRuntime)],
        destinationBucket: this.assetBucket,
        // Served under `/gen-ai/assets/pyodide/`; CloudFront strips the
        // `/gen-ai/assets/pyodide` prefix via the behavior's origin path, so
        // the key prefix here is just `pyodide/`.
        destinationKeyPrefix: 'pyodide',
        // Immutable, content-addressed: the long TTL is set on the behavior,
        // and this header makes a direct-origin fetch honest too.
        cacheControl: [
          CacheControl.setPublic(),
          CacheControl.maxAge(Duration.days(365)),
          CacheControl.immutable(),
        ],
        prune: true,
      });
    }

    this.errors = new Alarm(this, 'Errors', {
      metric: handler.metricErrors({ period: Duration.minutes(5) }),
      threshold: 1,
      evaluationPeriods: 1,
      comparisonOperator: ComparisonOperator.GREATER_THAN_OR_EQUAL_TO_THRESHOLD,
      // No traffic is the normal state on a route this quiet. Treating
      // missing data as breaching would alarm constantly.
      treatMissingData: TreatMissingData.NOT_BREACHING,
      alarmDescription: `GenAI lab handler errors (${props.stage}).`,
    });

    this.throttles = new Alarm(this, 'Throttles', {
      metric: handler.metricThrottles({ period: Duration.minutes(5) }),
      threshold: 1,
      evaluationPeriods: 1,
      comparisonOperator: ComparisonOperator.GREATER_THAN_OR_EQUAL_TO_THRESHOLD,
      treatMissingData: TreatMissingData.NOT_BREACHING,
      alarmDescription: `GenAI lab handler throttled (${props.stage}).`,
    });
  }
}
