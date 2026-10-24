# Phase 2 — Tasks

Status: in progress

## Done

- [x] Move the content document to `content/resume.json` as canonical
- [x] Full model types with `deny_unknown_fields` and camelCase
- [x] `careerBreaks` as a distinct type from `Role`
- [x] `ImpreciseDate` and `RoleEnd` with string wire form and manual `Ord`
- [x] `include_str!` embedding and `load()`
- [x] `validate()` returning all issues, distinguishing empty from placeholder
- [x] Tests against the real document: deserializes, reverse chronological,
      unique slugs
- [x] Date tests: both precisions, round trip, bad month, cross-precision order
- [x] `GET /api/resume`
- [x] Home page rendering employer and dates from the document

- [x] ETag and `Cache-Control`, with `If-None-Match` returning 304. Validator
      is a content fingerprint, not the commit SHA, so a redeploy that changes
      no content keeps caches warm
- [x] Build provenance: `GIT_SHA` and `BUILD_MONTH` baked in via `build.rs`
      rerun triggers, short SHA in the footer
- [x] `GET /api/projects`
- [x] Derived values: experience months and years, role spans, range labels
- [x] Timeline interleaving roles and career breaks, newest first
- [x] `Slug` newtype with validation, replacing `String`
- [x] Uniqueness tests across roles, breaks, skills, projects, certifications

- [x] Reconcile the summary prose with the derived figure. Settled on 15, and
      `validate()` now fails publishing if the two ever disagree, so the old
      site's stale-number failure cannot recur
- [x] JSON Schema export via `schemars` 1.x, served at `GET /api/schema` with
      its own ETag. `content_schema()` returns a `serde_json::Value` so the
      public API does not move when schemars does

## Next

- [ ] Decide whether `/api/*` responses carry `Vary` once theming lands.
      Deferred to Phase 3, where the cookie is introduced
- [ ] Express the parse rules for `Slug`, `ImpreciseDate`, and `RoleEnd` as
      JSON Schema patterns. They appear as plain strings now, which is true but
      loose. Worth doing when Phase 6 constrains model output, or if an
      external consumer needs it

## Blocked on content

Not code work. These are the Phase 0 carried-forward blanks, and the pipeline
will refuse to publish while any remain.

- [ ] Fill the placeholders in `content/resume.json`
