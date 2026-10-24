# Phase 0 — Design

Status: complete

The substance lives in `content-model.md` in this folder: normalization
decisions, the `resume-core` type sketch, the revision rationale, screening
considerations, and the remaining placeholders. This file records the decisions
that shaped later phases.

## Decisions

| Decision | Choice | Why |
| --- | --- | --- |
| Source of truth | Git, `content/` | Everything else must be rebuildable from a commit |
| Rendering | SSR, axum + maud, htmx | SEO and first paint matter; collapses three routing layers to one |
| IaC | CDK TypeScript, `cargo-lambda-cdk` | TypeScript is a daily driver; good Step Functions constructs; Rust build integrates with synth |
| Interactive feature | Job description fit analysis | A career walkthrough is a wizard with no decisions; misapplied orchestration undercuts the claim it was meant to support |
| HTTP entry | Lambda Function URL behind CloudFront | API Gateway offers nothing this needs |
| Session/flow state | DynamoDB, TTL | The only query pattern that justifies it |

## Three kinds of state

Established here because confusing them is how a portfolio ends up with content
that exists only in a database.

- Authored content: git. Canonical, reviewed, permanent.
- Derived artifacts: S3, keyed by commit SHA. Rebuildable, safe to delete.
- Flow sessions: DynamoDB with TTL. Ephemeral, never backed up.

## Rejected

- Career journey walkthrough as the interactive feature. See roadmap open
  question 7.
- WASM SPA. Sharing types into the browser is real, but SSR renders where the
  types already live.
- Terraform. Better market recognition, worse shipping speed, and learnable on
  an employer's infrastructure instead.
- SAM. Fine for plain Lambda, falls apart with CloudFront, DynamoDB, and three
  state machines across two stages.
