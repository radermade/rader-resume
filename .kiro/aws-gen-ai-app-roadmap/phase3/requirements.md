# Phase 3 — Policy assistant, end to end

Status: not started

Goal: author the EU policy assistant at full length — all of its steps, in
order, each with the decision, the snippets to assemble, the infrastructure,
the seeded workbench, and the intercepted responses. Phase 2 built the
mechanism and proved it against one call step (Retrieve, then Converse). Phase
3 is content: the other steps, authored to the same grain, so a visitor can
walk the whole build from the overview to the final failure.

## Re-evaluation against what phase 2 actually shipped

The roadmap wrote phase 3 as "all twenty-one steps." That number was set
before the mechanism existed; it is a target, not a contract. What phase 2
changed about the plan:

- The teaching model is snippet-assembly, not seed-the-answer. Each call step
  ships labelled snippets the visitor assembles into a scaffold, plus the
  intercepted responses. Every new step is authored in that shape.
- The lab runs real botocore, and phase 2 proved authored exception claims can
  be wrong: the original `maxTokens` and `global.` break-it exercises did not
  behave as written, because the first is optional and the second is validated
  server-side where the intercept never reaches. **Every step that claims an
  exception must be verified against real botocore before it ships.** This is
  the central discipline of phase 3, not an afterthought.
- Architecture icons are still deferred (phase 2 renders product names as
  text). Step 1 of every scenario is the diagram, so whether icons land is a
  real phase 3 decision, addressed below.
- The smoke test (`smoke-test.mjs`) exercises one assembled call end to end.
  Phase 3 extends it to cover the steps that make distinct, verifiable claims.

## The authored scenario

- R3.1 The policy assistant carries every step of its build, in order, from
  the overview through the empty-retrieval failure. The roadmap lists 21; the
  final count is whatever the build honestly has, with no padding and no step
  that is only a product name.
- R3.2 Step 1 is the architecture diagram, full width, no workbench — already
  built. Phase 3 does not change it except as the icon decision below requires.
- R3.3 Every call step states the decision in the job's terms and names the
  product that loses and why losing it walks toward the `Breaks when`. A step
  whose prose is only "use service X" fails review.
- R3.4 Every call step ships labelled snippets to assemble, a scaffold seed,
  and the intercepted responses its snippets call. The existing build checks
  (no snippets, seed-is-the-answer, unreachable response) apply to all of them.
- R3.5 Every call step ships its infrastructure block in the CDK dialect
  `infra/` uses.
- R3.6 Reasons are authored only where the genuine error is correct but
  unhelpful, and each is keyed on the exception botocore actually raises —
  verified, not assumed.

## Content accuracy (the phase's hard constraint)

- R3.7 Every exception a step claims — in a reason, in NOTES, or in prose — is
  reproduced against the real vendored botocore before the step ships. If the
  claimed exception does not fire, the step is corrected to what does, or the
  claim is removed.
- R3.8 A step that depends on server-side validation the intercept cannot
  reach (residency prefixes, quota, throttling) either teaches that as a
  stated sandbox boundary or is reframed to a client-side-checkable mistake.
  The `global.` case in phase 2 is the template: the sandbox cannot show it,
  so NOTES says so rather than pretending.
- R3.9 The smoke test is extended to assert the verifiable claims of the new
  steps — at minimum, every step whose lesson rests on a specific exception
  gets a smoke assertion. The browser-verify checklist covers the UI, the
  smoke test covers the Python, and no exception claim ships unverified by one
  of the two.

## The diagram and icons

Decided: ship the AWS Architecture Icons on step 1. They build product
familiarity, which is a direct aim of a cert-study tool — a learner should
come to recognize the Bedrock, S3, and Lambda marks on sight. The
`DiagramNode.icon` field already exists and is optional, so this is content,
not a migration.

- R3.10 Icons are the official AWS Architecture Icons from the AWS toolkit,
  used unmodified to create the diagram — the use AWS's icon terms describe
  ("create architecture diagrams"). Conditions that must hold: sourced from
  the official toolkit (not a third-party icon repo), not altered, not used in
  a way that implies AWS endorsement, and the lab carries a short attribution
  note. The product name is always rendered as the node's accessible label, so
  the icon is decoration over text, never the only identifier.
- R3.11 Icons are vendored like any asset: content-hashed, served from the lab
  origin under the immutable cache, not a CDN. Each icon is `aria-hidden`; the
  product name is the accessible label, so the diagram reads identically to a
  screen reader with or without icons.
- R3.12 A node whose feature has no icon of its own (Knowledge Bases) uses its
  parent product's icon with the full feature name, per the roadmap.
- R3.13 The diagram must still read on a phone and under the lab's CSP: no
  per-node inline styles, geometry stays in the hashed style block (the phase
  1/2 rule), and an SVG icon is referenced in a way the policy allows
  (an `<img>`/`<svg>` from `'self'`, covered by `img-src`/the existing style
  hash, with no inline style attribute introduced).
- R3.14 Note the commercial grey area: the IP license frames icon use "in
  connection with your permitted use of the Services," and this is a public
  portfolio. For a personal study/portfolio site this fits AWS's stated intent;
  if the lab ever became a commercial product, the terms get re-read. Recorded
  so the decision is revisitable, not silently assumed safe forever.

## Constraints carried forward

- R3.15 `gen-ai-core` still has no `aws-sdk-*` or `lambda_*`; scenarios stay
  content embedded at build time.
- R3.16 Steps are not addressable; advancing is a posted form value, and the
  workbench survives Next via the swap. No change to the routing or the
  session-free model.
- R3.17 No resume link until the full scenario walks on the deployed route.
  Then one link, per the roadmap.

## Exit criteria

Walking the policy assistant from the landing shows the diagram first with no
workbench, then a distinct decision, snippet set, infrastructure block, and
seeded workbench on each later step. Every exception the scenario claims fires
for real, verified by the smoke test or a recorded browser run. The coverage
intent of the step table holds: each listed decision is a real step, none is a
bare service name. The icon decision is made and implemented. None of it costs
a Lambda invocation beyond serving the page.
