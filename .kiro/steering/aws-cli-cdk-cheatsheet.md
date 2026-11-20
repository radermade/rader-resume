---
inclusion: manual
---

# AWS CDK, AWS CLI, and cargo-lambda Cheat Sheet

General-purpose reference for the three command-line tools. Not tied to any one
project. Pull it in with `#aws-cli-cdk-cheatsheet` when you need a command.

Conventions below: `<angle>` is a placeholder, `[square]` is optional.

---

# Part 1 — AWS CDK

CDK is infrastructure as code that synthesizes to CloudFormation. You write
constructs in a programming language; `cdk` turns them into templates and
drives CloudFormation to deploy them.

## Mental model

- **App** → one or more **Stacks** → **Constructs** → CloudFormation resources.
- `synth` produces a **cloud assembly** (`cdk.out/`): templates + asset
  metadata. Everything else operates on that assembly.
- CDK never edits resources directly. It diffs your synthesized template
  against the deployed CloudFormation stack and applies a **change set**.
- An **environment** is an `{ account, region }` pair a stack deploys to.

## Install and version

```bash
npm install -g aws-cdk          # global CLI
npx cdk --version               # or run the project-local one with npx
cdk --version
```

Keep the CLI (`aws-cdk`) and the library (`aws-cdk-lib`) roughly in step. A CLI
far newer than the library is usually fine; the reverse can warn.

## The core loop

```bash
cdk bootstrap                   # one-time per account/region (see below)
cdk synth                       # synthesize to cdk.out/, print the template
cdk diff                        # what would change vs. the deployed stack
cdk deploy                      # apply the change set
cdk destroy                     # tear the stack down
```

## bootstrap

Creates the `CDKToolkit` stack: an S3 bucket for assets, an ECR repo for Docker
image assets, and the IAM roles CDK assumes to deploy (`cdk-*-deploy-role`,
`-file-publishing-role`, `-lookup-role`, `-image-publishing-role`). Required
once per account+region before the first deploy.

```bash
cdk bootstrap                                   # current account/region from creds
cdk bootstrap aws://<account>/<region>          # explicit
cdk bootstrap aws://111122223333/us-east-1 aws://111122223333/us-east-2
cdk bootstrap --qualifier <q>                   # custom resource name qualifier
cdk bootstrap --toolkit-stack-name <name>       # non-default toolkit stack name
cdk bootstrap --cloudformation-execution-policies arn:aws:iam::aws:policy/AdministratorAccess
cdk bootstrap --trust <trusted-account-id>      # cross-account deploy trust
```

Notes:
- The default qualifier is `hnb659fds`. Roles are named with it; a custom
  qualifier must match `@aws-cdk/core:bootstrapQualifier` in the app context.
- CloudFront certificates must be in `us-east-1`, so a CloudFront app usually
  bootstraps both `us-east-1` and the primary region.

## synth

```bash
cdk synth                       # all stacks (or the single stack if one)
cdk synth <StackName>           # one stack's template to stdout
cdk synth <StackA> <StackB>     # several, ordered by dependency
cdk synth --quiet               # write cdk.out/ without printing YAML
cdk synth --context key=value   # pass context (see Context below)
cdk synth --all                 # every stack in the app
```

`synth` needs an account/region only if a stack reads environment-specific
data (e.g. a context lookup). Pure synthesis of an env-agnostic stack needs no
credentials.

## diff

```bash
cdk diff                        # diff all stacks against deployed state
cdk diff <StackName>
cdk diff --context key=value
cdk diff --app cdk.out          # diff a pre-synthesized assembly (no re-synth)
```

Diff output: `[+]` add, `[-]` remove, `[~]` modify. A `[~]` on a resource with
a replacement-triggering property is the thing to read carefully — it may
destroy and recreate.

## deploy

```bash
cdk deploy                      # deploy all (prompts on IAM/security changes)
cdk deploy <StackName>
cdk deploy <StackA> <StackB>
cdk deploy --all
cdk deploy --require-approval never     # skip the IAM-change prompt (CI)
cdk deploy --require-approval any-change
cdk deploy --app cdk.out                # deploy a pre-synthesized assembly
cdk deploy --context stage=prod
cdk deploy --outputs-file out.json      # write CfnOutputs to a file
cdk deploy --hotswap                    # dev-only: patch Lambda/code without CFN
cdk deploy --no-rollback                # keep resources on failure (faster iter)
cdk deploy --exclusively <Stack>        # deploy only this stack, not deps
cdk deploy --method=direct              # skip change set, call CFN directly
cdk deploy --parameters Key=Value       # pass a CFN parameter
cdk deploy --concurrency 4              # deploy independent stacks in parallel
```

`--require-approval`: CDK prompts before deploying changes that broaden IAM or
security-group access. `never` is for CI; interactively, read what it asks.

`--hotswap` / `--hotswap-fallback`: for fast inner-loop dev. Bypasses
CloudFormation for supported changes (Lambda code, Step Functions definitions,
ECS task defs). Never use in production — it drifts the stack from its
template.

Synthesize-once-deploy-twice pattern (avoids rebuilding assets):

```bash
cdk synth <Stacks> --quiet
# inspect / run checks against cdk.out/
cdk deploy <Stacks> --app cdk.out --require-approval never
```

## destroy

```bash
cdk destroy <StackName>
cdk destroy --all
cdk destroy --force             # no confirmation prompt
```

Resources with `RemovalPolicy.RETAIN` (often S3 buckets, data stores) survive a
destroy and must be deleted manually. Orphaned retained resources are a common
cause of a later "already exists" deploy error.

## import — adopt existing resources into a stack

For a resource that exists in the account but is not managed by this stack
(the classic "already exists" collision). Brings it under CDK without
recreating it.

```bash
cdk import <StackName>          # interactive: prompts for each resource's identifier
cdk import <StackName> --record-resource-mapping map.json   # non-interactive
```

Workflow: add the construct to your code so it is in the synthesized template,
then `cdk import` and supply the physical id (role name, bucket name, etc.).
After import, a normal `cdk deploy` reconciles the resource's properties to
your code. This is the standards-compliant alternative to deleting an orphan or
editing it in the console.

## migrate — generate CDK from existing infrastructure

```bash
cdk migrate --stack-name <name> --from-stack              # from an existing CFN stack
cdk migrate --stack-name <name> --from-path template.yaml # from a template file
cdk migrate --stack-name <name> --from-scan new           # scan the account's resources
```

Produces a new CDK project approximating existing resources. A starting point,
not a finished migration — review the output.

## Context

Context is key/value input resolved at synth time. Precedence, highest first:

1. `--context key=value` on the command line
2. `cdk.json` → `"context": { ... }`
3. `cdk.context.json` (cached lookups — committed so synth is reproducible)
4. `~/.cdk.json` (user-global)

```bash
cdk context                     # list cached context
cdk context --reset             # clear a cached entry (forces re-lookup)
cdk context --clear             # clear all cached context
```

Cached context is how environment lookups (VPC ids, AZs, AMI ids) stay
deterministic. Delete `cdk.context.json` entries when the underlying resource
changes and you want a fresh lookup.

## list, metadata, doctor, acknowledge

```bash
cdk ls                          # list stack names in the app
cdk ls --long                   # with environment info
cdk metadata <StackName>        # stack metadata
cdk doctor                      # environment / config sanity check
cdk acknowledge <notice-id>     # silence a specific CLI notice
cdk notices                     # show active notices (security, breaking)
```

## watch (dev inner loop)

```bash
cdk watch <StackName>           # redeploy on file change (uses hotswap when it can)
```

Reads the `"watch"` block in `cdk.json` for include/exclude globs. Dev only.

## The `--app` flag and multiple apps

`cdk` runs the command in `cdk.json`'s `"app"` by default. To run a different
entry point (e.g. a separate bootstrap/CI app):

```bash
cdk --app "npx ts-node bin/other-app.ts" deploy <Stack>
cdk --app cdk.out deploy <Stack>        # deploy an already-synthesized assembly
```

A separate app for one-time/bootstrap infrastructure (CI roles, DNS delegation)
is a common pattern: it has a different lifecycle and should not require the
main app's build toolchain to synthesize.

## Specifying the environment

```bash
cdk deploy --context account=111122223333 --context region=us-east-2
# or rely on the stack's env from CDK_DEFAULT_ACCOUNT / CDK_DEFAULT_REGION,
# which the CLI injects from the current credentials.
```

An env-agnostic stack (`env` unset) deploys to whatever the creds resolve to.
An env-specific stack pins `{ account, region }` in code and is deterministic
regardless of loaded creds — the safer default for anything real.

## Common flags (most subcommands)

```
--profile <name>        use a named credentials profile
--region <region>       override region
--role-arn <arn>        assume this role to perform the CloudFormation ops
--toolkit-stack-name    non-default bootstrap stack name
--output <dir>          cloud assembly dir (default cdk.out)
--verbose / -v          more logging (repeat for more)
--debug                 full debug output
--ci                    CI mode (no prompts, machine-friendly)
--no-color
```

## Reading deploy failures

- `ROLLBACK_COMPLETE` / `UPDATE_ROLLBACK_COMPLETE`: the deploy failed and CFN
  reverted. Read the **first** failure event, not the rollback cascade. The
  CloudFormation console events, or `aws cloudformation describe-stack-events`,
  show the root cause.
- `already exists`: a resource with that physical name exists outside the
  stack. Either `cdk import` it, delete the orphan, or let CFN auto-name it
  (drop the explicit name).
- `current credentials could not be used to assume cdk-...-role ... Proceeding
  anyway`: your principal can't assume the bootstrap roles (common as an admin
  or root), so CDK falls back to your own credentials. Usually harmless if your
  principal has the permissions; a sign the bootstrap trust is off otherwise.
- `No credentials`: no resolvable account. Load creds (`aws sts
  get-caller-identity` must succeed) or pass `--context account=`.

---

# Part 2 — AWS CLI

The AWS CLI (`aws`) is the general-purpose tool for every service. Below is the
machinery plus the high-traffic commands. Service coverage is enormous; this is
the spine.

## Install and configure

```bash
aws --version
aws configure                   # interactive: key, secret, region, output
aws configure --profile <name>  # a named profile
aws configure list              # show resolved config and where each came from
aws configure list-profiles
```

Config files:
- `~/.aws/credentials` — `aws_access_key_id`, `aws_secret_access_key`,
  `aws_session_token`.
- `~/.aws/config` — `[profile name]` sections: region, output, SSO, role
  assumption.

Credential resolution order (highest first): command-line flags → env vars →
`~/.aws/credentials` → `~/.aws/config` → container/IMDS (roles on
EC2/ECS/Lambda).

## Identity and sanity

```bash
aws sts get-caller-identity     # who am I — account, user/role ARN, user id
aws sts get-caller-identity --query Account --output text
```

The first thing to run when anything auth-related is confusing. If this fails,
nothing else will.

## SSO / IAM Identity Center

```bash
aws configure sso               # set up an SSO profile
aws sso login --profile <name>  # get a session (opens browser)
aws sso logout
```

## Assuming a role

```bash
aws sts assume-role \
  --role-arn arn:aws:iam::111122223333:role/MyRole \
  --role-session-name mysession
# returns temporary creds; export them or use a profile with role_arn +
# source_profile in ~/.aws/config, which the CLI assumes automatically.
```

## Global flags (every command)

```
--profile <name>        named profile
--region <region>       override region
--output <fmt>          json | text | table | yaml | yaml-stream
--query <jmespath>      filter/reshape output (see below)
--no-cli-pager          don't pipe output through a pager
--debug                 full wire-level trace
--endpoint-url <url>    hit a custom endpoint (localstack, VPC endpoint)
--no-sign-request       anonymous (public S3 objects, etc.)
--cli-read-timeout / --cli-connect-timeout
--page-size N           items per API page (not total)
--max-items N           cap total items returned (handles pagination for you)
--starting-token <tok>  resume pagination
```

## --query (JMESPath) — the high-value skill

Filters and reshapes JSON client-side. Learn this; it replaces most `grep`/`jq`.

```bash
# a single field
aws sts get-caller-identity --query 'Account' --output text

# a field from each item in a list
aws ec2 describe-instances \
  --query 'Reservations[].Instances[].InstanceId'

# filter a list by a condition
aws ec2 describe-instances \
  --query "Reservations[].Instances[?State.Name=='running'].InstanceId"

# reshape into a table of named columns
aws ec2 describe-instances \
  --query 'Reservations[].Instances[].{id:InstanceId,type:InstanceType,ip:PrivateIpAddress}' \
  --output table

# sort, length, contains
--query 'sort_by(Stacks, &StackName)[].StackName'
--query 'length(Buckets)'
```

`--query` is client-side (after the response). `--filters` (on services that
support it) is server-side and cheaper for large result sets — prefer it when
available.

## Pagination

Most `list`/`describe` commands paginate. The CLI auto-paginates unless output
is a pager. Control it:

```bash
aws s3api list-objects-v2 --bucket b --max-items 100
aws s3api list-objects-v2 --bucket b --page-size 1000 --max-items 5000
```

`NextToken`/`--starting-token` for manual paging.

## Service greatest hits

### STS / account
```bash
aws sts get-caller-identity
aws iam list-account-aliases
aws organizations describe-organization
```

### S3 (high-level `s3` vs. low-level `s3api`)
```bash
aws s3 ls                                   # buckets
aws s3 ls s3://bucket/prefix/ --recursive
aws s3 cp file.txt s3://bucket/key
aws s3 cp s3://bucket/key file.txt
aws s3 cp ./dir s3://bucket/prefix --recursive
aws s3 sync ./dir s3://bucket/prefix        # only changed files
aws s3 sync s3://bucket/prefix ./dir --delete
aws s3 rm s3://bucket/key
aws s3 presign s3://bucket/key --expires-in 3600   # temporary URL

# s3api: the full surface
aws s3api head-object --bucket b --key k
aws s3api list-object-versions --bucket b
aws s3api get-bucket-policy --bucket b
aws s3api put-object --bucket b --key k --body file --content-type text/html
```

### CloudFormation (what CDK drives under the hood)
```bash
aws cloudformation describe-stacks --stack-name <name>
aws cloudformation describe-stack-events --stack-name <name> \
  --query 'StackEvents[?ResourceStatus==`CREATE_FAILED` || ResourceStatus==`UPDATE_FAILED`]'
aws cloudformation list-stacks \
  --stack-status-filter CREATE_COMPLETE UPDATE_COMPLETE
aws cloudformation describe-stack-resources --stack-name <name>
aws cloudformation get-template --stack-name <name>
aws cloudformation delete-stack --stack-name <name>
aws cloudformation list-exports             # cross-stack exported values
```

`describe-stack-events` filtered to `*_FAILED` is the fastest way to find why a
CDK deploy rolled back.

### Lambda
```bash
aws lambda list-functions --query 'Functions[].FunctionName'
aws lambda get-function --function-name <name>
aws lambda invoke --function-name <name> --payload '{"k":"v"}' \
  --cli-binary-format raw-in-base64-out out.json
aws lambda update-function-code --function-name <name> --zip-file fileb://f.zip
aws lambda get-function-configuration --function-name <name>
aws lambda list-function-url-configs --function-name <name>
```

`--cli-binary-format raw-in-base64-out` is required for inline JSON payloads in
AWS CLI v2, or the payload is treated as base64 and fails.

### CloudWatch Logs (debugging Lambda / anything)
```bash
aws logs tail /aws/lambda/<fn> --follow                 # live tail
aws logs tail /aws/lambda/<fn> --since 1h
aws logs tail <group> --format short --filter-pattern 'ERROR'
aws logs describe-log-groups --query 'logGroups[].logGroupName'
aws logs start-query --log-group-name <g> \
  --start-time <epoch> --end-time <epoch> \
  --query-string 'fields @timestamp, @message | sort @timestamp desc | limit 20'
aws logs get-query-results --query-id <id>
```

`logs tail --follow` is the day-to-day debugging command.

### IAM
```bash
aws iam get-role --role-name <name> --query 'Role.AssumeRolePolicyDocument'
aws iam list-attached-role-policies --role-name <name>
aws iam get-role-policy --role-name <name> --policy-name <p>
aws iam list-roles --query 'Roles[?contains(RoleName, `deploy`)].RoleName'
aws iam list-open-id-connect-providers
aws iam get-open-id-connect-provider --open-id-connect-provider-arn <arn>
aws iam simulate-principal-policy --policy-source-arn <arn> \
  --action-names s3:GetObject --resource-arns <arn>   # will this be allowed?
```

`get-role ... AssumeRolePolicyDocument` is how you inspect a trust policy — the
thing to check when `AssumeRoleWithWebIdentity` (OIDC) is denied.

### Step Functions
```bash
aws stepfunctions list-state-machines
aws stepfunctions start-execution --state-machine-arn <arn> --input '{}'
aws stepfunctions start-sync-execution --state-machine-arn <arn> --input '{}'  # Express
aws stepfunctions describe-execution --execution-arn <arn>
aws stepfunctions get-execution-history --execution-arn <arn> --max-items 50
aws stepfunctions send-task-success --task-token <tok> --task-output '{}'
aws stepfunctions send-task-failure --task-token <tok> --error E --cause C
```

### DynamoDB
```bash
aws dynamodb list-tables
aws dynamodb describe-table --table-name <t>
aws dynamodb get-item --table-name <t> --key '{"id":{"S":"abc"}}'
aws dynamodb put-item --table-name <t> --item '{"id":{"S":"abc"},"n":{"N":"1"}}'
aws dynamodb query --table-name <t> \
  --key-condition-expression 'id = :id' \
  --expression-attribute-values '{":id":{"S":"abc"}}'
aws dynamodb scan --table-name <t> --max-items 25
```

### CloudFront
```bash
aws cloudfront list-distributions \
  --query 'DistributionList.Items[].{id:Id,domain:DomainName,alias:Aliases.Items}'
aws cloudfront create-invalidation --distribution-id <id> --paths '/*' '/index.html'
aws cloudfront get-distribution --id <id>
```

Invalidations: prefer targeted paths over `/*`. The first 1000 paths/month are
free; wildcards count as one path but invalidate broadly.

### Secrets Manager / SSM Parameter Store
```bash
aws secretsmanager get-secret-value --secret-id <name> --query SecretString --output text
aws ssm get-parameter --name /path/to/param --with-decryption \
  --query Parameter.Value --output text
aws ssm get-parameters-by-path --path /app/ --recursive --with-decryption
```

### EventBridge / Scheduler
```bash
aws events list-rules
aws events put-events --entries '[{"Source":"x","DetailType":"y","Detail":"{}"}]'
aws scheduler list-schedules
```

### ECR (for container images)
```bash
aws ecr get-login-password --region <r> | docker login --username AWS --password-stdin <acct>.dkr.ecr.<r>.amazonaws.com
aws ecr describe-repositories
aws ecr list-images --repository-name <repo>
```

### Service Quotas / Budgets (cost-adjacent)
```bash
aws service-quotas get-service-quota --service-code bedrock --quota-code <code>
aws service-quotas list-service-quotas --service-code bedrock
aws budgets describe-budgets --account-id <id>
```

### Bedrock (model calls)
```bash
aws bedrock list-foundation-models --query 'modelSummaries[].modelId'
aws bedrock-runtime invoke-model --model-id <id> --body <json> out.json \
  --cli-binary-format raw-in-base64-out
aws bedrock-agent-runtime retrieve --knowledge-base-id <id> \
  --retrieval-query '{"text":"..."}'
```

## Useful patterns

```bash
# capture an output into a shell variable
ACCOUNT=$(aws sts get-caller-identity --query Account --output text)

# wait for a resource state (built-in pollers)
aws cloudformation wait stack-create-complete --stack-name <name>
aws s3api wait bucket-exists --bucket <b>
aws lambda wait function-updated --function-name <fn>

# dry-run a mutating EC2 call to test permissions without doing it
aws ec2 run-instances --dry-run ...

# generate a skeleton input for a complex command
aws dynamodb put-item --generate-cli-skeleton
```

`aws <service> wait <condition>` pollers are underused and remove hand-rolled
sleep loops.

---

# Part 3 — cargo-lambda

`cargo-lambda` builds, tests, and deploys Rust functions for AWS Lambda. It
cross-compiles to the Lambda runtimes and packages the `provided.al2023`
custom-runtime bootstrap.

## Install

```bash
# via pip/pipx (what CI images often use)
pipx install cargo-lambda
# via cargo
cargo install cargo-lambda
# via homebrew
brew tap cargo-lambda/cargo-lambda && brew install cargo-lambda
```

It needs a cross-compile backend: **Zig** (default, `pip install ziglang` or
bundled) or the system cross toolchain. Zig is the least-friction option.

## new — scaffold

```bash
cargo lambda new <project>              # interactive: HTTP vs event function
cargo lambda new <project> --http       # an HTTP (API GW / Function URL) handler
cargo lambda new <project> --event-type sqs::SqsEvent
```

## build

```bash
cargo lambda build                      # debug build for Lambda
cargo lambda build --release            # optimized
cargo lambda build --release --arm64    # Graviton (cheaper/faster, common default)
cargo lambda build --release --x86-64
cargo lambda build --output-format zip  # produce a .zip instead of the dir layout
cargo lambda build --compiler cargo     # use host cargo cross instead of zig
cargo lambda build --target aarch64-unknown-linux-gnu
```

Output lands in `target/lambda/<binary>/bootstrap`. The binary is named
`bootstrap`, which is what `provided.al2023` expects.

Multiple binaries (a workspace with several Lambda targets):

```bash
cargo lambda build --release --arm64 --bin <binary-name>
```

## watch — local dev server

```bash
cargo lambda watch                      # runs the function locally on :9000, hot-reloads
cargo lambda watch --release
```

Emulates the Lambda runtime API locally. Combine with `invoke` to exercise the
handler without deploying.

## invoke — local or remote

```bash
# local (against cargo lambda watch)
cargo lambda invoke --data-ascii '{"k":"v"}'
cargo lambda invoke <fn> --data-file event.json
cargo lambda invoke --data-example apigw-request   # built-in example payloads

# remote (a deployed function)
cargo lambda invoke --remote <function-name> --data-file event.json
```

`--data-example <name>` pulls a real example event shape (API Gateway, SQS, S3,
etc.) so you don't hand-write envelopes.

## deploy

```bash
cargo lambda deploy                     # create/update the function from the last build
cargo lambda deploy <function-name>
cargo lambda deploy --iam-role arn:aws:iam::...:role/<role>
cargo lambda deploy --env-var KEY=value --env-var OTHER=thing
cargo lambda deploy --memory 512 --timeout 30
cargo lambda deploy --enable-function-url     # attach a Function URL
cargo lambda deploy --binary-name <name>      # pick a binary in a multi-bin crate
```

`cargo lambda deploy` is a quick path for standalone functions. In an
IaC-managed project you usually do **not** use it — the IaC (CDK, SAM, etc.)
owns the function, and `cargo lambda build` just produces the artifact the IaC
packages. Mixing `cargo lambda deploy` with IaC causes drift.

## With CDK (`cargo-lambda-cdk`)

When CDK's `RustFunction` construct is used, CDK invokes `cargo lambda build`
during synth/bundling. You rarely call cargo-lambda directly then; relevant
points:

- The construct resolves the crate by a `manifestPath` to its `Cargo.toml`.
  Relative paths resolve from the CDK app's working directory, so run `cdk`
  from the infra directory (or set `manifestPath` absolutely) or the build
  fails with "is not a path to a Cargo.toml file".
- The build honors `--release` and the target architecture the construct
  requests; set the function's architecture in CDK to match (`arm64`).
- `cargo-lambda` must be on `PATH` in the environment running `cdk synth`
  (locally and in CI — hence `pipx install cargo-lambda` in CI before deploy).

## Release profile for small, fast Lambdas

In `Cargo.toml`, the usual size/cold-start profile:

```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
panic = "unwind"   # let the runtime catch panics per-invocation
```

## Troubleshooting

- **"is not a path to a Cargo.toml file"** (via CDK): wrong working directory
  for a relative `manifestPath`. Run `cdk` from the infra dir or make the path
  absolute.
- **linker / cross-compile errors**: missing Zig or cross toolchain. `cargo
  lambda build` with `--compiler cargo` to use host cross, or install ziglang.
- **"ignoring deprecated linker optimization setting"** warning: benign linker
  noise from the Zig backend; not a failure.
- **function runs locally, fails deployed**: architecture mismatch (built x86,
  function configured arm64, or vice versa) or a missing env var that
  `watch`/`invoke` had locally.

---

# Quick decision notes

- **Deploy infrastructure** → CDK (`cdk deploy`). Not `aws cloudformation
  deploy` by hand, not `cargo lambda deploy` for IaC-managed functions.
- **Inspect / debug a live resource** → AWS CLI (`aws logs tail`, `aws
  cloudformation describe-stack-events`, `aws iam get-role`).
- **Build a Rust Lambda artifact** → `cargo lambda build --release --arm64`
  (directly, or via CDK's RustFunction which calls it for you).
- **"already exists" on deploy** → `cdk import`, or delete the orphan. Don't
  edit it in the console.
- **Auth confusion** → `aws sts get-caller-identity` first, always.
