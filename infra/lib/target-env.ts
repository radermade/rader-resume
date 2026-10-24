import { App, Environment } from 'aws-cdk-lib';

/**
 * Resolves the target account and region explicitly.
 *
 * Context wins over the `CDK_DEFAULT_*` variables the CLI injects, so a deploy
 * is deterministic rather than dependent on whichever credentials happen to be
 * loaded in the current shell. Deploying infrastructure to the wrong account
 * because a profile was exported is a bad afternoon.
 */
export function targetEnv(app: App): Environment {
  const account = app.node.tryGetContext('account') ?? process.env.CDK_DEFAULT_ACCOUNT;
  const region =
    app.node.tryGetContext('region') ?? process.env.CDK_DEFAULT_REGION ?? process.env.AWS_REGION;

  if (!account) {
    throw new Error(
      'No AWS account resolved. Either load credentials in this shell ' +
        '(aws sts get-caller-identity should succeed) or pass --context account=<id>.',
    );
  }

  if (!region) {
    throw new Error('No AWS region resolved. Set it in cdk.json context or pass --context region=<region>.');
  }

  return { account, region };
}
