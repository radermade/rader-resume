import { CfnOutput, Duration, RemovalPolicy, Stack, StackProps } from 'aws-cdk-lib';
import { ICertificate } from 'aws-cdk-lib/aws-certificatemanager';
import {
  AllowedMethods,
  CacheCookieBehavior,
  CacheHeaderBehavior,
  CachePolicy,
  CacheQueryStringBehavior,
  Distribution,
  HttpVersion,
  OriginRequestPolicy,
  PriceClass,
  SecurityPolicyProtocol,
  ViewerProtocolPolicy,
} from 'aws-cdk-lib/aws-cloudfront';
import { FunctionUrlOrigin, S3BucketOrigin } from 'aws-cdk-lib/aws-cloudfront-origins';
import { Architecture, FunctionUrlAuthType } from 'aws-cdk-lib/aws-lambda';
import { LogGroup, RetentionDays } from 'aws-cdk-lib/aws-logs';
import { AaaaRecord, ARecord, HostedZone, RecordTarget } from 'aws-cdk-lib/aws-route53';
import { CloudFrontTarget } from 'aws-cdk-lib/aws-route53-targets';
import { Alarm, TreatMissingData } from 'aws-cdk-lib/aws-cloudwatch';
import { SnsAction } from 'aws-cdk-lib/aws-cloudwatch-actions';
import { Topic } from 'aws-cdk-lib/aws-sns';
import { EmailSubscription } from 'aws-cdk-lib/aws-sns-subscriptions';
import { RustFunction } from 'cargo-lambda-cdk';
import { Construct } from 'constructs';
import { FitWorkflow } from './fit-workflow';
import { GenAiLab } from './gen-ai-lab';
import { ProvenanceWorkflow } from './provenance-workflow';

export interface SiteStackProps extends StackProps {
  readonly stage: string;
  /** Custom domain. Omit and the stack deploys with the Function URL only. */
  readonly domainName?: string;
  readonly hostedZoneId?: string;
  /** Must be a us-east-1 certificate. CloudFront accepts no other region. */
  readonly certificate?: ICertificate;
  /**
   * Subscribed to the alarm topic. Omit and the alarms deploy unrouted, which
   * the stack reports as an output rather than failing on.
   */
  readonly alarmEmail?: string;
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

    const fit = new FitWorkflow(this, 'Fit', { stage: props.stage });

    // A second binary on this distribution. The resume's routes, palette, and
    // policy are untouched; the lab owns its own.
    const lab = new GenAiLab(this, 'GenAiLab', { stage: props.stage });

    // Checks resolve this site's own artifacts against its public origin, so
    // without a domain there is nothing meaningful to check them against.
    const siteUrl = props.domainName ? `https://${props.domainName}` : undefined;

    const provenance = siteUrl
      ? new ProvenanceWorkflow(this, 'Provenance', { stage: props.stage, siteUrl })
      : undefined;

    const handler = new RustFunction(this, 'SiteHandler', {
      manifestPath: '../crates/site-handler/Cargo.toml',
      architecture: Architecture.ARM_64,
      memorySize: 256,
      timeout: Duration.seconds(10),
      logGroup,
      environment: {
        STAGE: props.stage,
        // Presence of these switches the handler from the local stand-ins to
        // the workflows. Absent locally, so development needs no AWS at all.
        FIT_STATE_MACHINE_ARN: fit.stateMachine.stateMachineArn,
        ...(provenance
          ? {
              PROVENANCE_STATE_MACHINE_ARN: provenance.stateMachine.stateMachineArn,
              PROVENANCE_TABLE: provenance.table.tableName,
            }
          : {}),
      },
    });

    // Synchronous for fit, because the handler waits for the answer. Async for
    // provenance, because it does not.
    fit.stateMachine.grantStartSyncExecution(handler);

    if (provenance) {
      provenance.stateMachine.grantStartExecution(handler);
      // The handler reads projections to render a page and claims a run to
      // deduplicate clicks. It never writes results; the task does.
      provenance.table.grantReadWriteData(handler);
    }

    // One topic for the stack. The constructs build their own alarms and the
    // stack decides where alerts land, so a workflow does not need to know an
    // email address exists.
    const alerts = new Topic(this, 'Alerts', {
      displayName: `rader-resume ${props.stage} alerts`,
    });

    if (props.alarmEmail) {
      alerts.addSubscription(new EmailSubscription(props.alarmEmail));
    }

    const alarms: Alarm[] = [
      fit.failedExecutions,
      lab.errors,
      lab.throttles,
      ...(provenance ? [provenance.failedExecutions] : []),
      // The handler erroring or being throttled is invisible to the workflow
      // alarms, since it is what calls them.
      new Alarm(this, 'HandlerErrors', {
        metric: handler.metricErrors({ period: Duration.minutes(5) }),
        threshold: 1,
        evaluationPeriods: 1,
        treatMissingData: TreatMissingData.NOT_BREACHING,
        alarmDescription: `Site handler errors (${props.stage}).`,
      }),
      new Alarm(this, 'HandlerThrottles', {
        metric: handler.metricThrottles({ period: Duration.minutes(5) }),
        threshold: 1,
        evaluationPeriods: 1,
        treatMissingData: TreatMissingData.NOT_BREACHING,
        alarmDescription: `Site handler throttled (${props.stage}).`,
      }),
    ];

    const action = new SnsAction(alerts);
    for (const alarm of alarms) {
      alarm.addAlarmAction(action);
    }

    new CfnOutput(this, 'AlertTopicArn', {
      description: props.alarmEmail
        ? 'Confirm the emailed subscription or alarms go nowhere'
        : 'No subscriber. Pass --context alarmEmail=<address> to route alarms',
      value: alerts.topicArn,
    });

    // Public read-only site. Nothing here mutates state.
    //
    // Still worth tightening: once CloudFront fronts this, the Function URL
    // should move to AWS_IAM auth with origin access control so the origin
    // cannot be reached directly, bypassing the cache and any edge controls.
    const url = handler.addFunctionUrl({
      authType: FunctionUrlAuthType.NONE,
    });

    new CfnOutput(this, 'FunctionUrl', { value: url.url });

    new CfnOutput(this, 'FitStateMachineArn', {
      description: 'For exercising the fit workflow directly',
      value: fit.stateMachine.stateMachineArn,
    });

    if (!props.domainName || !props.hostedZoneId || !props.certificate) {
      return;
    }

    // Short TTL with ETag revalidation. The handler emits a content
    // fingerprint, so a revalidation that finds no change costs a 304 rather
    // than a full response.
    //
    // Nothing is cached on cookies or query strings, which is deliberate:
    // theming is CSS-only precisely so the cache key stays simple. Adding a
    // cookie here would double the cache population.
    const shortCache = new CachePolicy(this, 'ShortCache', {
      defaultTtl: Duration.seconds(60),
      minTtl: Duration.seconds(0),
      maxTtl: Duration.minutes(5),
      enableAcceptEncodingGzip: true,
      enableAcceptEncodingBrotli: true,
      headerBehavior: CacheHeaderBehavior.none(),
      cookieBehavior: CacheCookieBehavior.none(),
      queryStringBehavior: CacheQueryStringBehavior.none(),
    });

    const origin = new FunctionUrlOrigin(url);

    // Forwarding the Host header to a Function URL origin breaks the request
    // signature, so it is excluded on every behavior.
    const originRequestPolicy = OriginRequestPolicy.ALL_VIEWER_EXCEPT_HOST_HEADER;

    // Endpoints that accept a POST or carry per-visitor state.
    //
    // CloudFront rejects any method a behavior does not list, before the
    // request reaches the origin, which is why the read-only default cannot
    // serve these. Caching is disabled rather than short: a fit report is
    // derived from one visitor's submission and a build stamp must never be
    // stale.
    const dynamic = {
      origin,
      viewerProtocolPolicy: ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
      allowedMethods: AllowedMethods.ALLOW_ALL,
      cachePolicy: CachePolicy.CACHING_DISABLED,
      originRequestPolicy,
      compress: true,
    };

    const labOrigin = new FunctionUrlOrigin(lab.url);

    // Same settings, different origin: the lab is its own function. POST is
    // allowed because advancing a step is a form post, and caching is
    // disabled because a step response is derived from the posted body.
    const labBehavior = {
      ...dynamic,
      origin: labOrigin,
    };

    // The lab's workbench assets are content-hashed and immutable. A long TTL
    // is safe because the filename changes when the bytes do, so the cache
    // never serves a stale asset. GET only: an asset is read, never posted.
    const immutableCache = new CachePolicy(this, 'LabAssetCache', {
      defaultTtl: Duration.days(365),
      minTtl: Duration.days(365),
      maxTtl: Duration.days(365),
      enableAcceptEncodingGzip: true,
      enableAcceptEncodingBrotli: true,
      headerBehavior: CacheHeaderBehavior.none(),
      cookieBehavior: CacheCookieBehavior.none(),
      queryStringBehavior: CacheQueryStringBehavior.none(),
    });

    const labAssets = {
      origin: labOrigin,
      viewerProtocolPolicy: ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
      allowedMethods: AllowedMethods.ALLOW_GET_HEAD,
      cachePolicy: immutableCache,
      originRequestPolicy,
      compress: true,
    };

    // The large runtime (Pyodide core, the wheels) is served from the lab's
    // S3 bucket with OAC, not the Lambda: it is multi-megabyte and binary, so
    // it does not belong in the function. The bucket keeps the whole payload
    // on this origin, so the workbench policy stays `'self'` with no CDN.
    //
    // `originPath` strips the `/gen-ai/assets/pyodide` request prefix so the
    // S3 key is just `pyodide/...`, matching the BucketDeployment prefix. OAC
    // and the read grant are created by this origin helper; the bucket itself
    // stays private with public access blocked.
    const labRuntimeOrigin = S3BucketOrigin.withOriginAccessControl(lab.assetBucket, {
      originPath: '/gen-ai/assets',
    });

    const labRuntime = {
      origin: labRuntimeOrigin,
      viewerProtocolPolicy: ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
      allowedMethods: AllowedMethods.ALLOW_GET_HEAD,
      cachePolicy: immutableCache,
      compress: true,
    };

    const distribution = new Distribution(this, 'Cdn', {
      comment: `${props.domainName} (${props.stage})`,
      domainNames: [props.domainName, `www.${props.domainName}`],
      certificate: props.certificate,
      defaultBehavior: {
        origin,
        viewerProtocolPolicy: ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        allowedMethods: AllowedMethods.ALLOW_GET_HEAD,
        cachePolicy: shortCache,
        originRequestPolicy,
        compress: true,
      },
      additionalBehaviors: {
        '/fit': dynamic,
        // Exact patterns, so '/fit' does not cover this. Without its own
        // behavior the POST falls to the read-only default, which allows only
        // GET and HEAD, and CloudFront rejects it before the origin sees it:
        // the modal would fail everywhere except local development.
        '/fit/fragment': dynamic,
        '/provenance': dynamic,
        '/provenance/*': dynamic,
        '/api/build': dynamic,
        // Both patterns, because CloudFront patterns are exact unless
        // wildcarded. `/gen-ai` alone would send `/gen-ai/policy-assistant`
        // to the read-only default, which allows only GET and HEAD, and
        // Previous and Next are posts: navigation would work locally and
        // fail at the edge. That is the trap `/fit/fragment` hit.
        // Assets first: a more specific pattern has to be ordered ahead of
        // the catch-all, or the immutable cache never applies and the runtime
        // is refetched on every visit. And within the asset paths, the
        // runtime prefix is more specific still and goes to S3, so it must
        // precede the Lambda-served `/gen-ai/assets/*`. CloudFront matches in
        // list order with no specificity sort, so this ordering is the rule,
        // not a hint: pyodide to the bucket, the small scripts to the Lambda,
        // everything else under the lab to the handler.
        '/gen-ai/assets/pyodide/*': labRuntime,
        '/gen-ai/assets/*': labAssets,
        '/gen-ai': labBehavior,
        '/gen-ai/*': labBehavior,
      },
      priceClass: PriceClass.PRICE_CLASS_100,
      httpVersion: HttpVersion.HTTP2_AND_3,
      minimumProtocolVersion: SecurityPolicyProtocol.TLS_V1_2_2021,
      enableIpv6: true,
    });

    const zone = HostedZone.fromHostedZoneAttributes(this, 'Zone', {
      hostedZoneId: props.hostedZoneId,
      zoneName: props.domainName,
    });

    const target = RecordTarget.fromAlias(new CloudFrontTarget(distribution));

    // Alias records, not CNAMEs. An apex domain cannot be a CNAME, which is
    // the whole reason DNS was delegated to Route 53.
    new ARecord(this, 'ApexA', { zone, target });
    new AaaaRecord(this, 'ApexAaaa', { zone, target });
    new ARecord(this, 'WwwA', { zone, recordName: 'www', target });
    new AaaaRecord(this, 'WwwAaaa', { zone, recordName: 'www', target });

    new CfnOutput(this, 'DistributionDomain', {
      description: 'Test against this before trusting the apex record',
      value: distribution.distributionDomainName,
    });
  }
}
