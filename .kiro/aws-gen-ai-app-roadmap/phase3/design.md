# Phase 3 — Design

Phase 3 is almost entirely authoring, not engineering. The types, the
renderer, the workbench, the intercept, and the build checks exist and are
proven. The work is writing nineteen more steps correctly and verifying every
claim they make against the real library. The design below is therefore about
the authoring process and the few mechanism changes the content forces, not a
new architecture.

## The authoring loop

Each step is written, then verified, then committed — in that order, because
phase 2 showed that an unverified exception claim is worse than no step.

1. **Write** the step in `policy-assistant.json`: decision, why, snippets,
   infrastructure, seeded scaffold, intercepted responses, and any reasons.
   Follow the shape of the existing Retrieve-then-Converse step.
2. **Verify** against real botocore using the reproduction harness. Assemble
   the snippets, run them, confirm the happy path prints the intercepted
   response. For any claimed exception, reproduce it and read the real
   traceback. If it does not fire as written, fix the step or the claim.
3. **Guard**: add a smoke-test assertion for the step's verifiable claim, so
   the claim cannot silently rot on a botocore bump.
4. **Commit** the step. Steps are independent content, so they commit
   independently and the scenario grows one reviewed step at a time.

The harness from phase 2 (the Node + Pyodide reproduction, now the committed
`smoke-test.mjs`) is the verification tool. It already extracts the intercept
from `worker.js` so it tests shipped code; extending it is adding assertions,
not rebuilding it.

## What each step's data looks like

Unchanged from phase 2 — the `CallStep` shape is fixed:

- `decision`, `why`: the prose. `why` names the losing product.
- `snippets`: labelled pieces to assemble. The build check requires at least
  one, and every intercepted operation must be reachable from a snippet.
- `infrastructure`: CDK for that step.
- `workbench.files`: the scaffold (not the answer), with `open` pointing at
  the file the visitor writes into.
- `workbench.responses`: keyed by service + operation, matched by the
  URL-based intercept (host label + trailing path segment, verified in
  phase 2).
- `workbench.reasons`: keyed on the real exception, only where the genuine
  error needs a sentence it cannot supply.

## Which steps carry a live call, and which are read-only

Not every step in the roadmap's 21 is a boto3 call. Many are design decisions
whose "code" is infrastructure or configuration, not a runtime API call:
chunking strategy, embedding dimension, metadata filter design, IAM policy,
PrivateLink, a versioned prompt template. Those steps still have the three
panes, but the workbench for them seeds a config/CDK artifact to read and
reason about, not a botocore call to run.

This is a real authoring decision per step: is the lesson something you *run*,
or something you *configure*? The roadmap's table already implies the split —
steps like 10 (Retrieve/Converse), 20 (throttling) are calls; steps like 3
(bucket Region), 6 (chunking), 13 (IAM) are configuration. A call step gets a
runnable workbench; a configuration step gets a workbench seeded with the
artifact and snippets that build it, and its "run" may just validate shape
rather than hit an intercepted transport.

The model already supports this: a step with responses runs against the
intercept; a step whose snippets are CDK/config simply has no botocore call to
intercept, and the lesson is reading and assembling the artifact. The build
check that "every intercepted operation is reachable from a snippet" is
vacuously satisfied when there are no responses — but such a step must then
not claim a runtime exception it cannot produce.

## Sandbox boundaries, stated per step

Phase 2's `global.` finding generalizes: the intercept replaces the network
hop, so anything validated server-side is invisible to the sandbox. Steps that
touch these must state the boundary rather than fake an error:

- **Residency / inference-profile prefixes** (`eu.` vs `global.`): accepted
  locally, rejected only at AWS. Teach as a boundary.
- **Throttling / quotas** (`ThrottlingException`, step 20): the sandbox never
  calls AWS, so there is no real throttle. This step teaches the *diagnosis*
  (check the account quota and the model, not a retry loop) from a seeded log
  or a described symptom, not a live exception.
- **IAM / permissions** (steps 8, 13): no credentials, no real authorization,
  so an access-denied cannot fire. These are configuration steps: the lesson
  is the policy document and the confused-deputy conditions, read and
  reasoned about, not a runtime `AccessDenied`.

What the sandbox *can* produce for real, and should be leaned on: parameter
validation (`ParamValidationError` for a genuinely required field), wrong
client / wrong method (`AttributeError` or operation errors), malformed
request shapes, and anything botocore checks before transport. Those are the
teachable live failures; the rest are read-and-reason.

## Icons

Decided: ship the official AWS Architecture Icons on step 1, for product
familiarity — a cert learner should come to recognize the marks. The
`DiagramNode.icon` field already exists, so this is additive content plus a
small renderer change.

Mechanism:

- **Source and vendor.** Download the official AWS Architecture Icons toolkit,
  take the SVGs for the products the policy assistant uses (Lambda, S3, API
  Gateway, Bedrock, Bedrock Knowledge Bases → Bedrock's mark, IAM), and vendor
  them under `assets/gen-ai/icons/` as a new embedded or served asset set,
  content-hashed, same immutable cache as the other lab assets. Unmodified.
- **`DiagramNode.icon`** holds the logical icon key (e.g. `lambda`,
  `s3`, `bedrock`). The renderer maps the key to the content-hashed URL, the
  same indirection the JS assets already use, so the markup references a stable
  name and the URL carries the hash.
- **Renderer.** The diagram node gains an `<img>` (or inline `<svg>`) above the
  caption. It is `aria-hidden`; the existing `.lab-node__caption` stays as the
  accessible label, so a screen reader is unaffected. The image is sized by a
  class in the hashed style block — no inline `style` attribute, per the phase
  1/2 CSP rule.
- **CSP.** An `<img src>` from this origin needs `img-src 'self'` on the lab
  pages that draw the diagram (the overview). That is a new directive, small
  and same-origin, added to the overview's policy only — the call-step policy
  does not draw the diagram. If the icons are inlined as `<svg>` instead of
  `<img>`, no `img-src` is needed but the SVG markup is in the hashed style/
  document and the style hash must cover it; the `<img>` path is simpler and
  the one directive is cheap. Decide at implementation; `<img>` is the default.
- **Fallback.** A node with no `icon` key (or an unknown one) renders the
  caption alone, exactly as today. So a missing icon degrades, it does not
  break, and the field staying optional means authoring an icon-less node is
  still valid.
- **Knowledge Bases** and other sub-features with no standalone icon use the
  parent product's icon key and the full feature name, per the roadmap.

Attribution: a short note in the lab footer or the stack page crediting the
AWS Architecture Icons, since the terms expect attribution and no implied
endorsement.

## Testing

- The smoke test gains one assertion per step that makes a verifiable claim:
  the happy path prints the intercepted response, and each claimed exception
  fires. It already extracts the intercept from `worker.js`, so new steps are
  new scenario fixtures and new assertions.
- `gen-ai-core` unit tests already enforce the structural invariants (snippets
  present, seed-not-answer, responses reachable, reasons well-formed) across
  all steps in the catalog — they run against every step automatically as the
  scenario grows. No new unit tests per step unless a step needs a new
  invariant.
- The browser-verify checklist gains the whole-scenario walk: diagram first,
  then each step distinct, workbench survives Next across all of them.
- `insta` snapshots (still pending from phases 1–2) become worth doing at full
  length: a snapshot of each rendered step catches an accidental content or
  layout regression as the scenario grows. This is the phase to add them.

## What phase 3 is not

Not the other scenarios — those are phase 4, authored at this same grain once
the policy assistant proves the full-length shape. Not persistence, not a
resume link beyond the single one the roadmap allows once the scenario walks,
not any change to routing, the session-free model, or the deploy path.
