# Phase 1 — Skeleton on `/gen-ai`

Status: not started

Goal: the route exists, serves the landing split, and opens the policy
assistant at its overview. Nothing is typeable yet.

This phase is deliberately a delivery mechanism with one page in it. The value
is proving the second binary, the two behaviors, and the local loop before any
scenario content depends on them.

## Functional

- R1.1 `crates/gen-ai-core` holds the scenario model and the markup. It has no
  `aws-sdk-*` and no `lambda_*` dependency, for the same reason `resume-core`
  does not: the logic stays unit-testable and the renderer stays callable from
  a future publish task.
- R1.2 `crates/gen-ai-handler` is one HTTP binary serving every route under
  `/gen-ai`. Routing lives there and nowhere else.
- R1.3 `GET /gen-ai` returns the landing split: the ASCII banner on the left,
  the project list on the right.
- R1.4 `GET /gen-ai/{id}` returns that scenario at step 1, the overview.
  Opening a project always starts at step 1.
- R1.5 Steps are not addressable. The URL stays `/gen-ai/{id}` for the whole
  project, which is what "the page does not remember where you were" means in
  practice. Advancing is a POST, not a navigation to a new address.
- R1.6 Previous and Next work without JavaScript, as a form POST returning the
  rendered step. htmx upgrades that to a swap of the lesson pane only, which
  is what will let phase 2's workbench state survive Next.
- R1.7 Step 1 renders with no workbench column. The workbench is phase 2.
- R1.8 One scenario document exists: the policy assistant, with its job brief,
  its step index, and step 1's overview. Steps 2 onward may be present as
  index entries without panes.
- R1.9 A scenario is an authored document in `content/gen-ai/`, embedded at
  build time with `include_str!`. The handler does not read `.kiro/` and does
  not read a file system at request time.
- R1.10 Scenario parsing is strict. An unknown field in an authored document
  is a typo and fails the build, matching the rule for `content/resume.json`.
- R1.11 An unknown scenario id renders the landing list, not an error page.
- R1.12 The resume binary gains no routes and no new dependency.
- R1.13 `cargo lambda watch` serves `/gen-ai` locally with no AWS credentials
  and no network access.

## Infrastructure

- R1.14 A second `RustFunction`: `provided.al2023`, `arm64`, 128 MB, 3 second
  timeout, explicit log retention.
- R1.15 Two CloudFront behaviors, `/gen-ai` and `/gen-ai/*`, pointed at the
  lab function URL on the existing distribution and certificate.
- R1.16 The behaviors allow POST and disable caching. Phase 1 already needs
  POST for Previous and Next, so this is used from the start rather than
  provisioned for phase 2.
- R1.17 The resume's default behavior is untouched, and so are the `/fit`,
  `/fit/fragment`, `/provenance`, and `/api/build` behaviors.
- R1.18 Alarms on the lab function's errors and throttles, on the existing
  alert topic.
- R1.19 The deploy job builds the lab binary through the same CDK construct
  path as the resume, in the same synth.

## Constraints

- R1.20 No model call on any request path, in this phase or any later one. A
  scenario is content.
- R1.21 No DynamoDB, no session, no saved position. The landing has no
  finished state and no retake. The step a visitor is on lives in the posted
  form and in the rendered page, nowhere else.
- R1.22 No Step Functions. Serving a page is not a workflow.
- R1.23 No link from the resume to `/gen-ai` until the route renders the
  overview on the deployed domain.
- R1.24 The lab's own Content Security Policy is as strict as the resume's: no
  `unsafe-inline`, no `unsafe-eval`, no wildcard. Any generated geometry is
  covered by the hashed style block rather than by `style` attributes.

## Palette

- R1.25 The lab defines its own token set, in its own stylesheet, owned by
  `gen-ai-core`. It shares no custom property with the resume. Neither app
  reads the other's palette, and neither is the other's fallback.
- R1.26 The lab's tokens are semantic, scale-based, and declared once at the
  root, the same discipline `design-standards.md` applies to the resume. A
  separate palette is not licence for hard-coded hex values in component
  rules.
- R1.27 Contrast is measured, not assumed: 4.5:1 for body text and 3:1 for
  large text and UI boundaries. Saturated colour on black is where this
  quietly fails.
- R1.28 Nothing encodes meaning by hue alone. The terminal's prompt is also a
  glyph and code is also monospace, so a visitor who cannot separate the two
  accents loses no information.
- R1.29 `design-standards.md` gains a short section recording that the lab has
  its own palette and why, so the divergence is a documented decision rather
  than something a later reader has to reverse-engineer.

## Deferred, with the reason

- The workbench, the runtime, and the intercept. Phase 2. Step 1 has no workbench by
  design, so the skeleton does not need one to be honest.
- AWS Architecture Icons, dashed region and VPC groups, and labelled
  orthogonal connectors. Phase 3's exit criteria name these; phase 1's names
  only "the row opens the overview". Phase 1 renders the overview's structure
  with named nodes and plain connectors, and leaves an icon slot in the model.
- In-page Python. Phase 3, and conditional on the runtime download being
  acceptable.
- The other scenarios and the coverage test. Phase 4.

## Settled

- **Steps are not addressable.** Deep linking is not a goal, so the roadmap's
  "at the step the browser asks for" is read as the posted step rather than a
  URL. This removes the conflict with workbench state entirely: a swap can
  preserve the workbench because there is no navigation to lose it, and no
  `hx-push-url` is needed because there is no address to keep honest.
- **The lab has its own palette**, not an exception to the resume's. See R1.25
  to R1.29.

## Open questions

- Q1 **Does the lab inherit the 100 KB first-view budget?** The resume's
  number exists because a recruiter opens it on a phone. A diagram-first
  technical lab, and later a Python runtime, is a different contract. State
  the lab's budget rather than inherit one by silence, so phase 3 has
  something to measure against.
- Q2 **AWS Architecture Icons terms.** Shipping the icon set is phase 3 work,
  but whether it may be shipped at all should be confirmed before the design
  depends on it. The fallback is named nodes, which phase 1 builds anyway, so
  this blocks nothing here.

## Exit criteria

`https://rader-resume.com/gen-ai` returns the split. Choosing the policy
assistant opens its overview with the job brief and the step index visible and
no workbench. `cargo lambda watch` serves the same pages locally. The resume's
routes, payload, and policy are byte-identical to before the phase.
