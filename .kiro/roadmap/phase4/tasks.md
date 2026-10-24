# Phase 4 — Tasks

Status: not started

## Decide first

- [ ] Trigger: direct `StartExecution` from CI, or S3 marker plus EventBridge
- [ ] Does the site handler serve pre-rendered artifacts, or keep rendering
      per request from embedded content? (see design)
- [ ] Dead external links: fail the publish, or warn only
- [ ] Does `dev` publish, or only `prod`

## Shared types

- [ ] Task payload types in `resume-core`, camelCase, no `deny_unknown_fields`
- [ ] Manifest type and its S3 layout

## Task binaries

- [ ] `tasks/ingest`: read content at SHA, normalize, write to S3
- [ ] `tasks/validate`: schema, `validate()`, link check, diagnostic on failure
- [ ] `tasks/render`: one artifact kind per invocation, driven by `Map`
- [ ] `tasks/publish`: write artifacts, flip manifest
- [ ] Failure path: diagnostic object, readable format

## Workflow

- [ ] ASL definition in CDK with JSONata expressions
- [ ] `Retry` with backoff and jitter on every task
- [ ] `Catch` on every task routing to the failure state
- [ ] Deterministic execution naming from the SHA
- [ ] Direct SDK integrations where a Lambda would only forward a call

## Infrastructure

- [ ] Artifacts bucket: versioned, private, SSE, lifecycle keeping current and
      previous generations
- [ ] Per-task IAM roles, no wildcards
- [ ] State machine logging and X-Ray
- [ ] `ExecutionsFailed` alarm

## CI

- [ ] Invoke the pipeline on merge to `main` with the merge SHA
- [ ] PR job runs validation and produces a preview artifact, publishes nothing

## Verify

- [ ] Re-run the same SHA twice, confirm identical result and no duplication
- [ ] Introduce a placeholder, confirm the publish fails before any write
- [ ] Roll back by re-pointing the manifest, confirm the live site follows
