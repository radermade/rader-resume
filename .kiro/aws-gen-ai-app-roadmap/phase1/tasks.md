# Phase 1 — Tasks

Status: code complete and green locally (fmt, clippy, tests, tsc). Not
deployed. Outstanding: the `design-standards.md` palette note, an `insta`
snapshot, a measured contrast check, and everything under "verify on the
deploy". `[~]` marks an item done differently than written, with the reason
inline.

## Settled

- [x] Steps are not addressable. Deep linking is not a goal, so a step is the
      posted form value and the URL stays `/gen-ai/{id}`. That removes the
      conflict between a linkable step and a workbench that survives Next,
      which was the only real design hole in the phase.
- [x] The lab has its own palette rather than an exception to the resume's.
      Its own stylesheet in `gen-ai-core`, sharing no custom property with
      `resume-core`.

## Settle first

- [ ] Q1, state the lab's page-weight budget. Moved to phase 2, where the
      Pyodide runtime forces the question and makes an answer meaningful. The
      lab's own budget is tracked in phase 2's tasks rather than guessed here.
- [ ] Q2, confirm the AWS Architecture Icons may be shipped before phase 3
      design depends on them. The fallback is named nodes, which this phase
      builds regardless, so this blocks nothing here.
- [ ] Add a palette section to `design-standards.md` recording that the lab
      has its own token set and why. Not yet done: the stylesheet exists and
      is self-consistent, but the steering still only describes the resume's
      palette, so the divergence is currently undocumented.

## Core

- [x] `crates/gen-ai-core`, no `aws-sdk-*` and no `lambda_*`. The CI check that
      enforces the ban lands in phase 2; the crate is clean now.
- [x] `Scenario`, `Brief`, `Step`, `Diagram` with `deny_unknown_fields` and an
      explicit `schemaVersion`.
- [x] `Step` as an enum with `Overview` and `Call` variants, so "the overview
      has no workbench" is a type-level fact rather than a runtime check.
- [x] `Step::as_call() -> Option<&CallStep>` as the single place the variant
      is checked.
- [x] A `Cursor` owning position, clamping an untrusted index once, with
      `next` and `previous` consuming `self`.
- [x] `Brief` as five named fields.
- [x] The failure field and the done field both required and both non-empty,
      with a test asserting they are not restatements of each other.
- [x] `catalog()` over the embedded documents, with the version checked.
- [x] Landing markup: ASCII banner, project list, each row a link.
- [x] Overview markup: job brief, step index with the current step marked,
      the diagram, and Previous and Next as submit buttons in a form posting
      the current step.
- [x] Diagram layout computed from the document. Levels derived from the graph
      rather than authored tiers. Geometry generated into the hashed style
      block, never into `style` attributes.
- [x] The lab's own stylesheet and token set. Semantic names, a spacing scale,
      no hard-coded hex in component rules, with a test that it shares no
      custom property with the resume.
- [~] Contrast of the chosen tokens. The values were picked off pure
      magenta/cyan with the ratios reasoned through in `design.md` (near 9:1
      and 13:1), but not measured against a computed-value checker. Measure
      before the link from the resume goes live.
- [x] Content Security Policy for the lab, hashing its own style, with a test
      asserting no `unsafe-` token and no wildcard. No script yet, so
      `script-src` is absent rather than a hash; that arrives in phase 2.

## Content

- [x] `content/gen-ai/policy-assistant.json`: the brief and step 1's overview
      nodes and groups.
- [x] The overview carries the system from the roadmap: company network into
      API Gateway, Lambda in the VPC, Bedrock and Knowledge Bases, S3 for the
      PDFs, IAM as the Lambda role.
- [~] Steps 2 onward as index entries. Currently only the retrieve-then-
      Converse step exists, now authored in full as phase 2's worked example
      rather than as a bare title. The other nineteen titles are not yet in
      the document, so the step index renders two steps, not twenty-one. This
      is honest for phase 1 but worth noting: the "all twenty-one" index is
      phase 3 work, not done here.

## Handler

- [x] `crates/gen-ai-handler`, axum `Router` behind `lambda_http::run`, the
      same shape as `site-handler`.
- [x] `GET /gen-ai`, `GET /gen-ai/{id}` always at step 1, and
      `POST /gen-ai/{id}` advancing to the posted step.
- [x] An unknown id redirects to the landing list. A step number outside the
      scenario clamps to the nearest valid step through `Cursor::at`.
- [x] The posted step clamped before use.
- [~] The landing is rendered once at cold start and served with an ETag.
      Step pages are rendered per request and sent `no-store`, not cached:
      the policy varies with the step's diagram geometry, so a shared cache
      keyed on the path would be wrong. The task as written assumed a single
      policy per page, which the per-page CSP hash makes false. Noted rather
      than forced.
- [x] Local serving over plain hyper on port 3001, so `cargo lambda watch`
      and a bare `cargo run` both work with no credentials.

## Infrastructure

- [x] A `GenAiLab` construct owning the function, log group, and alarms, and
      exposing the function URL.
- [x] `provided.al2023`, `arm64`, 128 MB, 3 second timeout, log retention set
      explicitly.
- [x] Behaviors for both `/gen-ai` and `/gen-ai/*`.
- [x] POST allowed and caching disabled on both.
- [x] Error and throttle alarms on the existing alert topic.
- [ ] Confirm the resume's behaviors, default behavior, and policy are
      unchanged by diffing the synthesized template. `tsc` passes and the
      construct is additive, but the template diff has not been run, because
      a clean `cdk synth` needs the account context and builds every binary.
      Do it with the deploy.

## Tests

- [x] Scenario document parses, at the expected version, with every brief
      field present and non-placeholder.
- [x] Every diagram edge names a node that exists.
- [x] The overview is the only step with no workbench, and every later step
      has one. The "index length matches declared count" check is subsumed by
      parsing: a scenario carries exactly the steps in `steps`, so there is no
      separate declared count to drift from. Phase 3 adds the count check when
      the coverage tables introduce one.
- [x] The overview renders no workbench, asserted rather than eyeballed.
- [x] Landing lists every scenario, and each row links to that scenario.
- [x] A posted step outside range clamps rather than panicking.
- [x] Previous and Next are a form that works without JavaScript.
- [x] The rendered page carries no `style` attribute and no `unsafe-` token in
      its policy.
- [ ] A snapshot of the landing and the overview. Not added: `insta` is not a
      dependency of `gen-ai-core` yet. The structural assertions above cover
      the behaviour; the snapshot is for catching unintended markup drift and
      is worth adding before the catalog grows in phase 4, when eyeballing
      stops scaling.

## Verify on the deploy

Not started. The lab has never been deployed; everything above was verified
locally, including a live `curl` against the handler on port 3001 showing the
split, the overview, a working POST advance, a 303 on an unknown id, and a
clamped out-of-range step. The edge-specific checks below cannot be reached
until a deploy happens, and that is gated on the account credentials the
resume deploy is also waiting on.

- [ ] `/gen-ai` returns the split on the deployed domain.
- [ ] The policy assistant row opens the overview.
- [ ] Next posts and renders the next step through CloudFront. The one thing
      that cannot be caught locally: a GET-only behavior works in
      `cargo lambda watch` and fails at the edge.
- [ ] The resume's pages are unchanged, including the published commit SHA and
      the `/stack` page.
- [ ] Note the added CI synth time from the second Rust binary.

## Not in this phase

Console, command checking, fixtures, in-page Python, AWS icons, the other
scenarios, the coverage test, and the link from the resume. Each has a phase
and a reason in `requirements.md`.
