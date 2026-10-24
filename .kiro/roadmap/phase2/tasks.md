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

## Next

- [ ] ETag and `Cache-Control` on read responses. Decide SHA versus content
      hash (see design)
- [ ] Surface the build commit SHA in the handler, for both ETag and the footer
- [ ] `GET /api/projects`
- [ ] JSON Schema export. Decide `schemars` versus hand-written
- [ ] Derived values: total years of experience, per-role tenure, formatted
      date ranges. Computed in `resume-core` with tests
- [ ] Interleave `careerBreaks` into the role timeline by date
- [ ] Slug newtype with validation, replacing `String`
- [ ] Extend uniqueness tests to skills, projects, certifications

## Blocked on content

Not code work. These are the Phase 0 carried-forward blanks, and the pipeline
will refuse to publish while any remain.

- [ ] Fill the placeholders in `content/resume.json`
