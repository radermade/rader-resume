# Phase 5 — Enrichment and Artifacts

Status: not started. Shape known, sources undecided.

Goal: the "living" part. Content refreshes without hand edits.

## Functional

- R5.1 A scheduled Standard workflow pulling from external sources.
- R5.2 Per-source fan-out with independent retry and partial-failure
  tolerance.
- R5.3 Fetched data is stamped with provenance and fetch time.
- R5.4 Derived data merges at publish rather than overwriting authored content.
- R5.5 When derived data should become permanent, the workflow opens a pull
  request against `content/`. Bots propose; humans merge.
- R5.6 PDF generation as its own branch, output to a stable URL.

## Constraints

- R5.7 Derived data never becomes the only copy of anything.
- R5.8 Derived data never silently overwrites an authored field.
- R5.9 `ToleratedFailurePercentage` set deliberately. One dead feed must not
  fail the run.
- R5.10 Schedules live in IaC alongside the state machine, via EventBridge
  Scheduler.
- R5.11 Credentials for external APIs come from Secrets Manager or Parameter
  Store, never IaC literals.

## Exit criteria

Site content refreshes on a schedule with no human in the loop, and the
downloadable PDF stays in sync.

## Open

- R5.12 Which sources, and at what frequency. GitHub activity is the obvious
  first one. Candidates: blog or feed items, speaking history, certification
  expiry.
- R5.13 Whether derived values like total years of experience belong here or
  are computed at render time in `resume-core`. Currently the latter.
