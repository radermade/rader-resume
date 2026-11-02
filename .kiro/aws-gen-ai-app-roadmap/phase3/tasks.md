# Phase 3 — Tasks

Phase 3 is authoring plus verification. The mechanism is done; the work is
writing the remaining steps and proving every claim. Steps are independent
content, so most of this list is per-step and the scenario grows one reviewed,
verified step at a time.

## Before authoring

Icons are decided: ship the official AWS Architecture Icons (R3.10–R3.14).
The mechanism is built; the real icon files are the remaining gate.

- [x] Icon asset registry in `gen-ai-core/src/assets.rs`: keyed icons
      (`lambda`, `s3`, `api-gateway`, `bedrock`, `iam`), `icon_url(key)`
      lookup, served content-hashed through `by_url` like the scripts.
- [x] `DiagramNode.icon` key mapped to the content-hashed URL in the renderer.
- [x] Icon rendered as an `aria-hidden` `<img>` above `.lab-node__caption`,
      sized by `.lab-node__icon` in the hashed style block (no inline style).
      `img-src 'self'` added to the overview page's CSP only (a page with a
      layout); the landing, which draws no diagram, does not take it. Tested.
- [x] `icon` keys wired into the policy-assistant diagram nodes; Knowledge
      Bases uses Bedrock's icon with the full feature name.
- [ ] Replace the placeholder SVGs in `assets/gen-ai/icons/` with the official
      AWS Architecture Icons toolkit SVGs (same filenames). Placeholders ship
      now so the mechanism is testable without redistributing AWS assets; they
      are visibly provisional. See `assets/gen-ai/icons/README.md`.
- [ ] Add an AWS Architecture Icons attribution note (footer or stack page)
      once the real icons are in.
- [ ] `insta` snapshot of the overview with icons, once the real icons land.
- [ ] Write a short AUTHORING.md beside the scenario: each step is written the
      same way — write → reproduce against real botocore → add smoke assertion
      → commit.

## The steps

The roadmap lists 21 decisions for the policy assistant. All are now authored:
1 overview + 20 call steps, parsing and validating, smoke test green. Grouped
below by what the sandbox can produce — a runnable call or a read-and-reason
configuration step.

### Runnable call steps (a real botocore call, verifiable)

- [x] Step 8: department metadata, filtered Retrieve. Filter shape verified
      against real botocore (validates, reaches transport, no
      ParamValidationError).
- [x] Step 9: incremental ingest — StartIngestionJob on the bedrock-agent
      control plane. Required params (knowledgeBaseId, dataSourceId) verified
      against real botocore; async STARTING status.
- [x] Step 11: citations — RetrieveAndGenerate returns answer + citations.
      Authored; shape matches the standard KNOWLEDGE_BASE config. (Add a smoke
      assertion for it — see Verification.)
- [~] Step 7 (embedding dimension) authored as a configuration step, not a
      runnable call: the mismatch failure is an ingestion-time error the
      sandbox cannot reproduce, so the lesson is the model/index declaration.

### Configuration / read-and-reason steps (no live call; seed the artifact)

Model change made this phase so config steps are honest: `Workbench.responses`
is now optional, a step with none is a configuration step (`CallStep::is_runnable`
is false), and the build check no longer forces a response onto it. Forcing a
fake response would have been dishonest. Tests and the smoke test updated to
find the runnable step rather than assume index 1.

- [x] Step 2: retrieval not fine-tune — the decision and the CDK that
      provisions a knowledge base rather than a customization job. Authored as
      a config step (no live call); the lesson is the absent
      CreateModelCustomizationJob.
- [x] Step 3: corpus bucket in an EU Region with a lifecycle. Config step.
- [x] Step 4: parse tables/headings before chunking (advanced parsing on the
      data source). Config step.
- [x] Step 5: a managed knowledge base, not a self-run store. Config step.
- [x] Step 6: hierarchical chunking, locked at data-source creation. Config.
- [x] Step 7: one embedding model, matching index dimension. Config.
- [x] Step 12: a guardrail for grounding and prompt-attack on a published
      version. Config step; DRAFT-vs-published is the lesson.
- [x] Step 13: IAM for the model/profile ARNs with the guardrail forced by a
      condition. Config step (no credentials in the sandbox, so no live
      AccessDenied); the policy shape is the lesson.
- [x] Step 14: PrivateLink interface endpoint for bedrock-runtime. Config.
- [x] Step 15: API Gateway in front, auth + throttle. Config.
- [x] Step 16: versioned prompt template (Prompt Management). Config.
- [x] Step 17: eval scores retrieval and generation separately. Config.
- [x] Step 18: invocation logs encrypted, Macie, guardrail trace off. Config.
- [x] Step 19: publish gate blocks on an eval drop. Config.

### Boundary / diagnosis steps (stated sandbox limits)

- [x] Step 10 addendum: the `eu.` vs `global.` residency note is carried as a
      stated boundary in the Retrieve/Converse NOTES. Pattern established.
- [x] Step 20: `ThrottlingException` as a diagnosis step. No real throttle in
      the sandbox; the lesson is the investigation order from a seeded symptom.
- [x] Step 21: empty retrieval — the final failure. Runnable: Retrieve returns
      an empty result set (the sandbox CAN show this), and the NOTES tie it to
      the scenario's `Breaks when`. The filter/ingest/embedding checklist is
      the lesson.

## Verification (per step, not optional)

- [~] Happy paths against real botocore: the new runnable calls (filtered
      Retrieve, StartIngestionJob, RetrieveAndGenerate, empty Retrieve) had
      their request shapes validated against real botocore (no
      ParamValidationError, params match). Not every step was run end to end,
      and config steps have no happy path to run.
- [x] Claimed exceptions reproduced: the only exception claims in the scenario
      are the Retrieve/Converse step's (missing `modelId` → ParamValidationError,
      wrong client → AttributeError), both verified against real botocore in
      phase 2. The new steps make no new exception claims, by design — they are
      config and diagnosis steps.
- [ ] NOT DONE: smoke-test assertions for the new runnable operations. The
      smoke test still covers only the original Retrieve/Converse call plus the
      config-step boot case. Filtered Retrieve, StartIngestionJob,
      RetrieveAndGenerate, and the empty-Retrieve result have no committed
      guard. This is the real remaining verification gap, and it matters more
      for phase 4 than the content itself.
- [ ] NOT DONE: `insta` snapshots of each rendered step.

## Done

- [x] All 21 steps authored (1 overview + 20 calls), parsing and validating,
      41/41 unit tests, smoke test green. Each step has a distinct decision,
      snippets, infrastructure, and workbench; config steps seed an artifact,
      runnable steps intercept a response.
- [x] Every step is a real step with a decision, none a bare service name
      (enforced by the brief/step tests and reviewed in authoring).
- [x] Every exception the scenario claims is verified (only the Retrieve/
      Converse step claims any; verified phase 2).
- [ ] NOT DONE: the full scenario walked in a browser — diagram first,
      workbench surviving Next across all 21 steps. Author/unit side is done;
      the live walk is a human pass.
- [ ] NOT DONE: real AWS Architecture Icons (placeholders ship now) and the
      attribution note.
- [ ] NOT DONE: smoke assertions for the new runnable operations (see
      Verification).
- [ ] NOT DONE: `insta` snapshots.
- [ ] NOT DONE: the scenario walked on the deployed route, and the single
      resume link added.

## Not in this phase

The other scenarios (phase 4), persistence, and any resume link beyond the one
the roadmap allows once the policy assistant walks end to end.
