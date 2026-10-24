# Phase 1 — Walking Skeleton

Status: complete

Goal: one trivial Rust Lambda, deployed by CI, reachable over HTTPS.

## Functional

- R1.1 A cargo workspace that builds a Lambda-targetable Rust binary.
- R1.2 An HTTP handler returning a response over a public HTTPS URL.
- R1.3 A push to `main` builds, deploys, and produces that URL with no manual
  steps.

## Constraints

- R1.4 `provided.al2023` on `arm64`. Memory, timeout, and log retention set
  explicitly.
- R1.5 No long-lived AWS credentials anywhere. CI authenticates via GitHub
  OIDC.
- R1.6 The CI deploy role carries no permissions on application resources, only
  `sts:AssumeRole` on the CDK bootstrap roles.
- R1.7 Deploying the CI credential must not require the Rust toolchain.
- R1.8 `resume-core` has no `aws-sdk-*` or `lambda_*` dependency.
- R1.9 Routing uses axum. No hand-rolled matching on `lambda_http::Request`.

## Exit criteria

A push to `main` produces a live URL.

## Out of scope

Real content, visual design, custom domain, CloudFront.
