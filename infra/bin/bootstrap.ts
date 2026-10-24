#!/usr/bin/env node
/**
 * One-time bootstrap infrastructure, deliberately a separate CDK app.
 *
 * CDK synthesizes every stack in an app before deploying any of them, so
 * keeping this with the site stack meant deploying the CI deploy role required
 * a working Rust build toolchain. Bootstrap concerns have a different
 * lifecycle and should not depend on the application's.
 */
import { App, Tags } from 'aws-cdk-lib';
import { GithubOidcStack } from '../lib/github-oidc-stack';
import { targetEnv } from '../lib/target-env';

const app = new App();

const owner = app.node.tryGetContext('githubOwner');
const repo = app.node.tryGetContext('githubRepo');
const environment = app.node.tryGetContext('stage') ?? 'dev';

if (!owner || !repo) {
  throw new Error('githubOwner and githubRepo context values are required');
}

const oidc = new GithubOidcStack(app, 'rader-resume-github-oidc', {
  owner,
  repo,
  environment,
  existingProviderArn: app.node.tryGetContext('existingOidcProviderArn'),
  env: targetEnv(app),
});

Tags.of(oidc).add('project', 'rader-resume');
Tags.of(oidc).add('component', 'ci');
Tags.of(oidc).add('managed-by', 'cdk');
