# Phase 4 — Tasks

Author the rest of the catalog to the methodology. No new mechanism. Each
scenario is a JSON document that must pass the existing `gen-ai-core` guards and
tests. Work top to bottom; scenarios are independent and can be parallelized.

## Setup

- [ ] Confirm the embed list in `gen-ai-core` (how `catalog()` finds scenario
      documents) and the path convention `content/gen-ai/<id>.json`.
- [ ] Re-read the methodology in `phase4/requirements.md` R4.1–R4.17 before
      authoring. The guards enforce most of it; authoring to it first avoids a
      round-trip through a failing test.

## Per scenario (repeat for each of R4.18–R4.29)

For `<id>`:

- [ ] Brief: all five fields, `breaksWhen` and `doneMeans` distinct.
- [ ] Step 1: diagram with icons, groups, labelled edges, no dangling edge.
- [ ] Each step table row → a call step in pattern A (config), B (runnable),
      C (dual), or D (diagnosis). Name the losing product; tie it to
      `breaksWhen`.
- [ ] Every runnable step: a `shell` terminal exercise + its authored success
      `response` (a `get`/`list` control-plane call from the design's table).
- [ ] Every config step: present/absent `checks`, present patterns in the
      typescript snippets, absent patterns absent.
- [ ] Every exception claim reproduced against real botocore; corrected or
      removed if it does not fire.
- [ ] Register the document in the embed list; confirm `catalog()` loads it.
- [ ] `cargo test -p gen-ai-core` green for the new document (guards + tests).

## Order of work

- [ ] Case graph (`case-graph`, 6 steps)
- [ ] Employee answers (`employee-answers`, 5)
- [ ] Contract desk (`contract-desk`, 9)
- [ ] Product catalog search (`product-catalog-search`, 10)
- [ ] Member services (`member-services`, 13)
- [ ] Benefits agent (`benefits-agent`, 9)
- [ ] Desk router (`desk-router`, 8)
- [ ] Overnight claims (`overnight-claims`, 10)
- [ ] Support chat (`support-chat`, 14)
- [ ] Clinical intake (`clinical-intake`, 17)
- [ ] Brand voice (`brand-voice`, 13)
- [ ] Analyst workspace / Amazon Quick (`analyst-workspace`, step table from the
      roadmap Quick note)

## Terminal exercises (the audit rule, applied everywhere)

- [ ] Every runnable step across all scenarios has a shell snippet. The
      `RunnableStepHasNoShell` guard fails the build otherwise, so this is
      checked automatically — but author it deliberately as a success path, not
      a throwaway to satisfy the guard.
- [ ] Each shell snippet's operation has an authored response so the command
      runs green. Prefer a control-plane `get`/`list` that verifies the step's
      dependency.

## Cross-scenario tests

- [ ] `smoke-test.mjs`: one runnable step per scenario whose lesson rests on a
      specific exception gets a smoke assertion (python runs through the
      intercept, claimed exception or clean result asserted).
- [ ] Confirm the brief-completeness and step-coverage tests cover every new
      scenario (they fail the build on a missing field or an empty step row).
- [ ] Full gate green: `cargo test`, `cargo clippy --all-targets
      --all-features -- -D warnings`, `cargo fmt --check`, `node --check
      assets/gen-ai/workbench.js`.

## Landing

- [ ] The landing lists every built scenario by name; opening one starts it at
      step 1. (Already implemented; confirm each new `<id>` appears.)

## Out of scope (stated so it is not quietly added)

- Any new workbench surface, pane, or editor. The single editor + terminal is
  the settled shape.
- In-browser CDK synth, deploy, or typecheck. Configuration steps are
  structural-check only.
- A terminal answer key or `cli` block. The terminal just runs; botocore is the
  verification.
- Addressable steps, saved position, or a session model. Next is a posted form
  value.
- Any `aws-sdk-*`/`lambda_*` dependency in `gen-ai-core`. Scenarios are embedded
  content.
