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

## Deploy

- [x] `AWS_DEPLOY_ROLE_ARN` and `AWS_REGION` repo variables set
- [x] `dev` GitHub environment created
- [x] Pushed to `main`, CI built and deployed `rader-resume-dev`

Role ARN: `arn:aws:iam::<account>:role/rader-resume-github-deploy-dev`

## Follow-ups

Not Phase 1 blockers, but needed before the Phase 4 pull-request flow is real.

- [ ] Enable branch protection on `main`: no direct pushes, PR required, CI
      green required. The first push had to create the branch, which is why
      this comes after rather than before.
- [ ] Confirm the Function URL actually serves the page and `/api/resume`
      returns the document
- [ ] Root credential cleanup, tracked in `phase7/tasks.md`
