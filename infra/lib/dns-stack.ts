import { CfnOutput, Fn, RemovalPolicy, Stack, StackProps } from 'aws-cdk-lib';
import { PublicHostedZone } from 'aws-cdk-lib/aws-route53';
import { Construct } from 'constructs';

export interface DnsStackProps extends StackProps {
  readonly domainName: string;
}

/**
 * The hosted zone, deployed once from the bootstrap app.
 *
 * Deliberately not in the application stack. Deleting and recreating a hosted
 * zone issues a new set of nameservers, which means re-entering them at the
 * registrar and waiting out propagation again. Keeping it here, with a retain
 * policy, makes that hard to do by accident.
 *
 * Registration stays at the current registrar. Only DNS is delegated, because
 * an apex domain cannot be a CNAME and Route 53 alias records are what make
 * pointing `example.com` at CloudFront possible.
 */
export class DnsStack extends Stack {
  readonly hostedZone: PublicHostedZone;

  constructor(scope: Construct, id: string, props: DnsStackProps) {
    super(scope, id, props);

    this.hostedZone = new PublicHostedZone(this, 'Zone', {
      zoneName: props.domainName,
      comment: 'Delegated from the registrar. Managed by CDK.',
    });

    this.hostedZone.applyRemovalPolicy(RemovalPolicy.RETAIN);

    new CfnOutput(this, 'NameServers', {
      description: 'Set these as the custom nameservers at the registrar',
      value: Fn.join(', ', this.hostedZone.hostedZoneNameServers ?? []),
    });

    new CfnOutput(this, 'HostedZoneId', {
      value: this.hostedZone.hostedZoneId,
    });
  }
}
