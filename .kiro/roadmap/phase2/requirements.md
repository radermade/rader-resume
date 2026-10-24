# Phase 2 — Content Model and Storage

Status: in progress

Goal: the resume exists as typed, versioned data with a read path.

## Functional

- R2.1 The full resume document is expressed as Rust types in `resume-core`.
- R2.2 `content/resume.json` is the canonical document, in git.
- R2.3 Parsing is strict. An unknown field in authored content is a typo and
  must fail.
- R2.4 Validation reports every reason the document is not publishable, not
  just the first.
- R2.5 Unresolved `[bracketed]` placeholders are detectable as a distinct
  failure from empty values.
- R2.6 `GET /api/resume` serves the document as JSON.
- R2.7 Read responses carry an ETag and cache headers.
- R2.8 A JSON Schema can be exported from the Rust types.

## Constraints

- R2.9 Dates preserve source precision. 2006 is a year; inventing a month is a
  lie.
- R2.10 Ordering derives from dates, never array position.
- R2.11 Slugs are unique and stable.
- R2.12 Derived values (years of experience, tenure, "current" formatting) are
  computed, never stored. The old data hardcoded ten years against a 2006
  start and went stale.
- R2.13 Tests assert properties of the real content document, not fixtures.
- R2.14 Validation failures do not fail the unit suite. Work continues against
  a document with known gaps; publishing is what gets blocked.

## Exit criteria

Real resume content served as JSON from a deployed endpoint, with the schema
enforced by tests against the actual document.

## Out of scope

Rendering beyond a smoke-test page, PDF, variants, visual design.
