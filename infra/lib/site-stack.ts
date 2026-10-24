import { CfnOutput, Duration, RemovalPolicy, Stack, StackProps } from 'aws-cdk-lib';
import { Architecture, FunctionUrlAuthType } from 'aws-cdk-lib/aws-lambda';
import { LogGroup, RetentionDays } from 'aws-cdk-lib/aws-logs';
import { RustFunction } from 'cargo-lambda-cdk';
import { Construct } from 'constructs';

export interface SiteStackProps extends StackProps {
  readonly stage: string;
}

export class SiteStack extends Stack {
  constructor(scope: Construct, id: string, props: SiteStackProps) {
    super(scope, id, props);

    // Log retention is set explicitly. The Lambda default is never-expire,
    // which quietly becomes the largest line item on a project this size.
    const logGroup = new LogGroup(this, 'SiteHandlerLogs', {
      retention: props.stage === 'prod' ? RetentionDays.ONE_MONTH : RetentionDays.TWO_WEEKS,
      removalPolicy: RemovalPolicy.DESTROY,
    });

    const handler = new RustFunction(this, 'SiteHandler', {
      manifestPath: '../crates/site-handler/Cargo.toml',
      architecture: Architecture.ARM_64,
      memorySize: 256,
      timeout: Duration.seconds(10),
      logGroup,
      environment: {
        STAGE: props.stage,
      },
    });

    // Public by design: this serves the site. Nothing here mutates state.
    // Phase 3 puts CloudFront with OAC in front and this URL stops being the
    // front door. Any write endpoint added before then needs auth first.
    const url = handler.addFunctionUrl({
      authType: FunctionUrlAuthType.NONE,
    });

    new CfnOutput(this, 'SiteUrl', { value: url.url });
  }
}
