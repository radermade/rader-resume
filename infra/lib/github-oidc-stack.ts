import { CfnOutput, Duration, Stack, StackProps } from 'aws-cdk-lib';
import {
  Effect,
  OpenIdConnectPrincipal,
  OpenIdConnectProvider,
  PolicyStatement,
  Role,
} from 'aws-cdk-lib/aws-iam';
import { Construct } from 'constructs';

export interface GithubOidcStackProps extends StackProps {
  readonly owner: string;
  readonly repo: string;
  /** GitHub Actions environment allowed to assume the role. */
  readonly environment?: string;
  /**
   * An account can only have one GitHub OIDC provider. Pass the ARN of an
   * existing one to reuse it instead of failing on create.
   */
  readonly existingProviderArn?: string;
}

/**
 * Deployed once, manually, from local credentials. This is the one stack CI
 * cannot create, because it is what CI authenticates with.
 */
export class GithubOidcStack extends Stack {
  constructor(scope: Construct, id: string, props: GithubOidcStackProps) {
    super(scope, id, props);

    const provider = props.existingProviderArn
      ? OpenIdConnectProvider.fromOpenIdConnectProviderArn(
          this,
          'GithubProvider',
          props.existingProviderArn,
        )
      : new OpenIdConnectProvider(this, 'GithubProvider', {
          url: 'https://token.actions.githubusercontent.com',
          clientIds: ['sts.amazonaws.com'],
        });

    const environment = props.environment ?? 'dev';
    const subject = `repo:${props.owner}/${props.repo}:environment:${environment}`;

    const role = new Role(this, 'DeployRole', {
      roleName: `${props.repo}-github-deploy-${environment}`,
      description: 'Assumed by GitHub Actions via OIDC to run cdk deploy',
      maxSessionDuration: Duration.hours(1),
      assumedBy: new OpenIdConnectPrincipal(provider, {
        StringEquals: {
          // Both conditions matter. Without the aud check, any GitHub
          // repository's token could be presented. Without the exact sub,
          // any workflow in any repo could assume this role.
          'token.actions.githubusercontent.com:aud': 'sts.amazonaws.com',
          'token.actions.githubusercontent.com:sub': subject,
        },
      }),
    });

    // Least privilege for CDK: this role needs no permissions on application
    // resources, only the ability to assume the roles `cdk bootstrap` created.
    // Those roles carry the actual deploy permissions.
    role.addToPolicy(
      new PolicyStatement({
        effect: Effect.ALLOW,
        actions: ['sts:AssumeRole'],
        resources: [`arn:aws:iam::${this.account}:role/cdk-*`],
      }),
    );

    new CfnOutput(this, 'DeployRoleArn', { value: role.roleArn });
  }
}
