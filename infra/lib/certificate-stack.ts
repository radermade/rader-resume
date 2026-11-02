import { Stack, StackProps } from 'aws-cdk-lib';
import { Certificate, CertificateValidation, ICertificate } from 'aws-cdk-lib/aws-certificatemanager';
import { HostedZone } from 'aws-cdk-lib/aws-route53';
import { Construct } from 'constructs';

export interface CertificateStackProps extends StackProps {
  readonly domainName: string;
  readonly hostedZoneId: string;
}

/**
 * The CloudFront certificate, which must live in us-east-1 regardless of where
 * everything else runs. That is a hard CloudFront constraint, not a choice.
 *
 * The hosted zone is referenced by attributes rather than looked up, so no
 * credentials are needed at synth time and the lookup-role warning does not
 * appear. DNS validation writes CNAME records into the zone automatically and
 * will sit pending indefinitely if delegation has not propagated.
 */
export class CertificateStack extends Stack {
  readonly certificate: ICertificate;

  constructor(scope: Construct, id: string, props: CertificateStackProps) {
    super(scope, id, props);

    const zone = HostedZone.fromHostedZoneAttributes(this, 'Zone', {
      hostedZoneId: props.hostedZoneId,
      zoneName: props.domainName,
    });

    this.certificate = new Certificate(this, 'Certificate', {
      domainName: props.domainName,
      subjectAlternativeNames: [`www.${props.domainName}`],
      validation: CertificateValidation.fromDns(zone),
    });
  }
}
