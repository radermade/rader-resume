# Phase 6 — Visitor-Facing Interactive Features

Status: domain logic built for both features. Workflow, endpoints, and views
outstanding.

Two features, deliberately different in architecture because they have
different characteristics.

## 6A — Live provenance check (the state machine)

A visitor clicks a button and the system independently verifies what is
verifiable about the page in front of them, then reports back.

### Functional

- R6.1 `POST /provenance` starts a check run and redirects to its result page.
- R6.2 `GET /provenance/{id}` shows progress, then results.
- R6.3 Checks are derived from the content document: external links in the
  profile, project repositories and sites, published artifacts, and whether
  the commit the footer claims matches what the site reports.
- R6.4 Results distinguish three outcomes that must never be conflated:
  passed, unreachable, and mismatched. Only a mismatch implies something is
  wrong.
- R6.5 A GitHub repository check reports existence and last push date.
- R6.6 Repeated triggers within the refresh window return the cached run.

### Constraints

- R6.7 Naming and copy must state what this does not cover. It verifies links
  and public artifacts. It cannot verify employment, titles, or dates, and
  implying otherwise would undermine the point of having it.
- R6.8 An unreachable target is reported as "could not reach", never as a
  failed claim. Corporate sites block datacenter addresses routinely.
- R6.9 Individual check failures are tolerated and do not fail the run. Only
  the run breaking is a failure.
- R6.10 A hard ceiling on checks per run, so content growth cannot turn one
  click into a hundred outbound requests.
- R6.11 A minimum interval between fresh runs for the same plan. This is an
  outbound request amplifier aimed at third parties who did not opt in.
- R6.12 `GET /provenance/{id}` reads a projection and makes zero Step
  Functions calls.
- R6.13 Run identity is the fingerprint of the plan plus the commit, used as
  the execution name so duplicate triggers collide at the service level.
- R6.14 Progress is visible and honest: never reports complete while running.

### Why this justifies orchestration

A dozen or more outbound calls with timeouts will not finish inside a
reasonable request budget. Each target fails independently for reasons outside
this system, so per-branch retry with backoff and a tolerated failure
percentage are meaningful rather than decorative. The run is worth inspecting
afterwards. That is four of the five characteristics that justify a state
machine, which is four more than the rejected career walkthrough had.

### Exit criteria

A visitor triggers a run, watches it progress, and sees a result that
distinguishes unreachable from wrong. Repeated clicks inside the window reuse
the run. No third party receives more traffic than the ceiling allows.

## 6B — Job description fit analysis (Express state machine)

Deterministic throughout, orchestrated, and synchronous from the visitor's
point of view. Full design in `design-6b.md`.

- R6.15 `POST /fit` accepts pasted or dropped job description text and returns
  the result in the same response, via `StartSyncExecution` against an Express
  workflow.
- R6.16 Requirements are extracted by matching a skills taxonomy in
  `content/`, and each is classified by dimension: tech, skills, experience,
  or seniority.
- R6.17 Four `Parallel` branches, one per dimension, each matching against a
  different corpus slice with different weights. Branches must not duplicate
  each other's logic.
- R6.18 Aggregation produces a score, a tier band, matched requirements with
  supporting resume text, explicit gaps, and three to five emphasis points.
- R6.19 Emphasis points are the role highlights satisfying the most must-have
  requirements, deduplicated by role.
- R6.20 Requirements outside the taxonomy are dropped, and the count of
  dropped requirements is reported, so the result is honest about its coverage.
- R6.21 Citations are structural and must resolve, or the report is rejected.
- R6.22 A single failed branch yields a partial report rather than no report.
- R6.23 `/fit` works with a textarea and a form post, no JavaScript. A
  `<dialog>` modal with paste and dropped `text/plain` enhances it.
- R6.24 No file uploads.
- R6.25 UI copy states that the score reflects overlap with documented
  experience, not suitability.

## Deferred

- R6.21 A model-backed extraction and matching path behind the existing trait
  seam, if access and budget ever justify it.
- R6.22 File drag-and-drop with PDF and DOCX extraction.
- R6.23 Tailored resume variants, which belong in Phase 4's artifact fan-out.
