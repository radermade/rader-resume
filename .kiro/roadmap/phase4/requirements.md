# Phase 4 — Publishing Pipeline

Status: not started. Design is settled; task breakdown will sharpen on contact.

Goal: content changes flow through a reviewable, retryable workflow.

## Functional

- R4.1 A Standard state machine invoked by CI on merge to `main`, with the
  commit SHA as input.
- R4.2 Stages: resolve SHA, ingest, normalize, validate, merge derived data,
  render artifacts, write under a SHA prefix, flip the manifest, invalidate
  cache, notify.
- R4.3 Artifacts: HTML, JSON, PDF, Open Graph images.
- R4.4 Publishing is the atomic flip of a manifest pointer.
- R4.5 Rollback is re-pointing the manifest at an earlier SHA, with no rebuild
  and no redeploy.
- R4.6 Failures record a diagnostic readable without opening the console.

## Constraints

- R4.7 Validation rejects unresolved `[bracketed]` placeholders. A PDF reaching
  a recruiter containing `[Current Title]` is the worst outcome this project
  can produce.
- R4.8 Nothing is written before validation passes.
- R4.9 Execution names derive from the commit SHA, so duplicate triggers
  collide instead of double-publishing.
- R4.10 Re-running the same SHA is safe and produces the same result.
- R4.11 Every task has explicit `Retry` with backoff and jitter, and a `Catch`
  to the failure path.
- R4.12 Payloads carry identifiers and S3 keys, never documents.
- R4.13 Cache invalidation targets changed paths, not a wildcard.
- R4.14 The pipeline is the only writer. CI never mutates deployed state
  directly.

## Exit criteria

Merging a content PR publishes the change with no manual steps, and
re-pointing the manifest rolls it back.
