#!/usr/bin/env node
import { App, Tags } from 'aws-cdk-lib';
import { CertificateStack } from '../lib/certificate-stack';
import { SiteStack } from '../lib/site-stack';
import { targetEnv } from '../lib/target-env';

const app = new App();
const stage = app.node.tryGetContext('stage') ?? 'dev';
const domainName = app.node.tryGetContext('domainName');
const hostedZoneId = app.node.tryGetContext('hostedZoneId');

// Where alarms go. Context rather than a literal, because it is a personal
// address and content files carry no contact details. Absent, the topic and
// alarms still deploy and nothing subscribes, which the stack warns about.
const alarmEmail = app.node.tryGetContext('alarmEmail');

const env = targetEnv(app);
const withDomain = Boolean(domainName && hostedZoneId);

// CloudFront only accepts certificates from us-east-1, so this is a separate
// stack in that region. `crossRegionReferences` is what lets the site stack
// consume the certificate ARN across the boundary, and must be set on both.
const cert = withDomain
  ? new CertificateStack(app, 'rader-resume-cert', {
      domainName,
      hostedZoneId,
      env: { account: env.account, region: 'us-east-1' },
      crossRegionReferences: true,
    })
  : undefined;

const site = new SiteStack(app, `rader-resume-${stage}`, {
  stage,
  env,
  crossRegionReferences: true,
  domainName: withDomain ? domainName : undefined,
  hostedZoneId: withDomain ? hostedZoneId : undefined,
  certificate: cert?.certificate,
  alarmEmail,
});

Tags.of(site).add('component', 'site');
if (cert) {
  Tags.of(cert).add('component', 'cert');
}

Tags.of(app).add('project', 'rader-resume');
Tags.of(app).add('stage', stage);
Tags.of(app).add('managed-by', 'cdk');
