# Living Resume & Portfolio — Roadmap

Rust on AWS Lambda, content-as-data, Step Functions for the workflows that are
genuinely stateful.

This file is the index. Each phase has its own folder with `requirements.md`,
`design.md`, and `tasks.md`. Detail lives there; this file tracks status and
decisions so they do not drift into two places.

## Status

| Phase | Name | Status |
| --- | --- | --- |
| [0](phase0/) | Discovery and decisions | Complete |
| [1](phase1/) | Walking skeleton | Complete pending first CI deploy |
| [2](phase2/) | Content model and storage | In progress |
| [3](phase3/) | Public site | Not started |
| [4](phase4/) | Publishing pipeline | Designed, not started |
| [5](phase5/) | Enrichment and artifacts | Outlined |
| [6](phase6/) | Fit analysis (flagship) | Designed, not started |
| [7](phase7/) | Admin, hardening, operations | Outlined |
| [8](phase8/) | Polish | Outlined, deliberately thin |

Phases 7 and 8 are outlines on purpose. Designing them before the system exists
produces documents that get rewritten.

## Immediate next actions

1. Set `AWS_DEPLOY_ROLE_ARN` and `AWS_REGION` repo variables, create the `dev`
   GitHub environment, push to `main`, confirm a live URL. Closes Phase 1.
2. ETag and commit SHA surfacing, then derived values. See `phase2/tasks.md`.
3. Fill the content placeholders listed in `phase0/tasks.md`. Not code work,
   but the pipeline will refuse to publish while they remain.

## Guiding principles

- Content is data, not markup. The resume is a versioned document; the site is
  one of several renderings.
- "Living" means automated refresh, not hand edits.
- Lambda-first, no always-on compute.
- Step Functions where there is orchestration, retry, or fan-out. A state
  machine used as glue reads as over-engineering and argues against its author.
- One deploy path. If it is not in IaC and CI, it does not exist.
- Three kinds of state, never confused: authored content in git, derived
  artifacts in S3 keyed by SHA, ephemeral sessions in DynamoDB with a TTL.

## Repo shape

```
crates/
  resume-core/      model, validation, rendering, payload types
                    no aws-sdk-* or lambda_* dependencies
  site-handler/     one HTTP binary: site, read API, fit endpoints
  tasks/            one binary per Step Functions task state
infra/
  bin/app.ts        application stacks
  bin/bootstrap.ts  one-time bootstrap, no Rust dependency
content/            canonical resume document and assets
.kiro/              roadmap, steering
```

One app, one workspace, several build targets.

## Decisions

Recorded so they are not relitigated. Reasoning is in the relevant
`design.md` and in `.kiro/steering/`.

| Question | Decision |
| --- | --- |
| Source of truth | Git. `content/` is canonical; everything else rebuildable from a commit |
| Rendering | SSR from Lambda, axum + maud, htmx. No WASM SPA |
| IaC | CDK TypeScript with `cargo-lambda-cdk` |
| HTTP entry | Function URL behind CloudFront. No API Gateway, no WebSockets |
| Interactive feature | Job description fit analysis. A career walkthrough was rejected |
| Flow state | DynamoDB with TTL, read via a projection, never by querying executions |
| Region | `us-east-2`, with the CloudFront certificate in `us-east-1` |

## Open questions

1. How much of the old site's content survives beyond the resume data? Full
   migration or rewrite of the project write-ups.
2. Admin UI, or editing entirely through commits and pull requests?
   Git-as-truth points at the latter, which would shrink Phase 7 substantially.
3. Which external sources feed the enrichment sync, and how often?
4. Which Bedrock model for requirement extraction, and what does the eval set
   look like?
5. Abuse thresholds: input size cap, per-IP rate limit, daily execution
   ceiling, monthly Bedrock budget. Numbers before launch, not after the first
   bill.
6. Domain name and registrar.
7. Does the site handler serve pre-rendered artifacts after Phase 4, or keep
   rendering per request from embedded content?
