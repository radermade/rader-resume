# Phase 1 — Tasks

Status: one task remaining

## Scaffold

- [x] `rust-toolchain.toml` pinned to 1.99.1 with the arm64 target
- [x] Cargo workspace, edition 2024, `unsafe_code = "forbid"`, release profile
- [x] `resume-core` crate with no AWS or Lambda dependency
- [x] `site-handler` crate: axum router behind `lambda_http::run`, maud page
- [x] `.gitignore`

## Infrastructure

- [x] `SiteStack`: RustFunction on arm64, explicit log group, Function URL
- [x] Four required tags applied at the app level
- [x] `GithubOidcStack`: provider plus deploy role scoped by `aud` and `sub`
- [x] Split bootstrap into its own CDK app
- [x] `targetEnv()` for explicit account and region
- [x] `cdk bootstrap` run against the account
- [x] `rader-resume-github-oidc` deployed

## CI

- [x] `check` job: fmt, clippy with warnings denied, test, infra typecheck
- [x] `deploy` job: OIDC, cargo-lambda via pipx, targets `rader-resume-dev`

## Local verification

- [x] `cargo test` passes
- [x] `cargo clippy --all-targets -- -D warnings` clean

## Remaining

- [ ] Set `AWS_DEPLOY_ROLE_ARN` and `AWS_REGION` repo variables, create the
      `dev` GitHub environment, commit and push to `main`, confirm a live URL

Role ARN: `arn:aws:iam::<account>:role/rader-resume-github-deploy-dev`
