#!/usr/bin/env node
import { App, Tags } from 'aws-cdk-lib';
import { SiteStack } from '../lib/site-stack';
import { targetEnv } from '../lib/target-env';

const app = new App();
const stage = app.node.tryGetContext('stage') ?? 'dev';

const site = new SiteStack(app, `rader-resume-${stage}`, {
  stage,
  env: targetEnv(app),
});

Tags.of(site).add('component', 'site');

Tags.of(app).add('project', 'rader-resume');
Tags.of(app).add('stage', stage);
Tags.of(app).add('managed-by', 'cdk');
