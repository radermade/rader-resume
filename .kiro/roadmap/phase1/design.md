# Phase 1 — Design

Status: implemented

## Layout

```
Cargo.toml                 workspace, shared dep versions, lints, release profile
rust-toolchain.toml        pinned 1.99.1, arm64 target
crates/resume-core/        model and logic, no AWS or Lambda deps
crates/site-handler/       the one HTTP binary
infra/bin/app.ts           application app: SiteStack
infra/bin/bootstrap.ts     bootstrap app: GithubOidcStack
infra/lib/target-env.ts    explicit account/region resolution
.github/workflows/ci.yml   check on PR, check + deploy on main
```

## Two CDK apps, not one

CDK synthesizes every stack in an app before deploying any of them. With the
OIDC stack in the same app, deploying the CI credential triggered a
cargo-lambda build of the site handler and failed on a missing Docker fallback.

Bootstrap infrastructure has a different lifecycle than the application and
should not depend on the application's toolchain. `bin/bootstrap.ts` is a
separate app invoked through `npm run deploy:oidc`.

## Credential bootstrapping

CI cannot create its own credentials, so the OIDC stack is hand-deployed once.
Everything after that flows through commits, satisfying the pipeline rule that
the pipeline is the only writer.

The role's entire security boundary is its trust policy. It holds no
application permissions, so the conditions matter:

- `aud` must equal `sts.amazonaws.com`, or any GitHub repo's token would do.
- `sub` must equal `repo:<owner>/<repo>:environment:<stage>` exactly, which is
  why the GitHub environment must exist.

## Environment resolution

`targetEnv()` prefers CDK context over the `CDK_DEFAULT_*` variables the CLI
injects, so a deploy cannot silently follow whichever profile happens to be
exported. Region is committed in `cdk.json`; account is not, because an account
ID in a public repo is free reconnaissance for role-name enumeration.

## Version findings

Worth recording, because both cost a build cycle:

- `lambda_http` must be 1.x. The 0.5 line's `run` required
  `Error = lambda_http::Error`; an axum `Router` has `Error = Infallible` and
  can never satisfy it. 1.x relaxed the bound to `E: Debug + Into<Diagnostic>`.
- `lambda_http::tracing` is feature-gated. Use `tracing_subscriber` directly,
  configured for JSON, no ANSI, no timestamps.
