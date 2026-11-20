# Phase 4 — Design

How to author the rest of the catalog without building anything new. The design
is a set of authoring patterns and a per-scenario checklist, because the
mechanism is done: phase 3 shipped the model and renderer, phase 3.5 collapsed
to the single editor and the always-on terminal, and the audit added the
`RunnableStepHasNoShell` guard. Phase 4 is applying that shape, repeatedly and
accurately.

## What is reused verbatim

- `Scenario::parse` and all its guards: no-overview, dangling-edge, duplicate
  title, no-seed-files, opens-missing-file, config-step-has-no-checks,
  no-snippets, unknown-language, runnable-has-no-python, runnable-has-no-shell.
- The `gen-ai-core` tests: answer-key integrity (checks satisfiable from the
  typescript snippets), operation reachability (python method or CLI kebab),
  scaffold-not-the-answer, reasons-can-fire, brief-completeness.
- The renderer: single editor, action-follows-extension, terminal on runnable
  steps, diagram with icons.
- The worker intercept: matches on service + operation (case-insensitive), so
  an authored `response` for `{ service, operation }` answers both the python
  call and the CLI form.

## The authoring patterns

Each scenario is a JSON document under `content/gen-ai/<id>.json`, embedded at
build. The patterns below are the shapes a step takes; every step is one of
them.

### Pattern A — configuration step (the common case)

A decision you configure, not call. Opens a `.ts` (or `policy.ts`, `eval.ts`,
`pipeline.ts`), assembles typescript snippets, graded by checks.

- `responses: []`, `checks: [ ...present/absent pairs ]`.
- At least one present check names the construct that must appear; at least one
  absent check names the losing anti-pattern (wildcard IAM, fine-tune job,
  DRAFT in production). The absent checks are where half the teaching lives.
- Every present check's pattern must appear in a typescript snippet (the
  integrity test enforces this); every absent check's pattern must not.
- No shell snippet (not runnable, no terminal).

### Pattern B — runnable step (a live call)

A decision you make by running a call. Opens a `.py`, assembles python
snippets, teaches by running real botocore.

- `responses: [ ...bodies for every operation the snippets call ]`.
- Python snippets build the client and the call; the open `.py` is a stub.
- Exactly the methodology's terminal rule: at least one `shell` snippet whose
  operation has an authored `response`, so the terminal command succeeds.
  The control-plane `get-*`/`list-*` that verifies the step's dependency is the
  pattern — e.g. a step that Retrieves pairs with a terminal
  `list-data-sources` or `get-knowledge-base`.
- Optional `reasons` where the genuine exception is correct but unhelpful,
  keyed on the exception botocore actually raises (verified, R4.15).

### Pattern C — dual step (runnable + a stack to check)

A step that both runs a call and checks the infrastructure behind it. The
policy assistant's "Retrieve, then Converse" is the one worked example; use it
sparingly, where a decision genuinely has both a call and a stack worth
practicing.

- `responses` (makes the `.py` runnable) AND `checks` (grade the `.ts`).
- Seeds both a `.py` and a `.ts`; `open` is the `.py`. The single editor tracks
  both; opening the `.ts` turns the button into Check.
- Still needs its shell snippet (it is runnable). The terminal command, the
  python call, and the stack check are three facets of the one decision.

### Pattern D — diagnosis step

A failure with no live call to make (throttling, an IAM denial, a cost
estimate, an empty retrieval that is the point). Two sub-shapes:

- D1, a readable checklist: opens a `.md`/`.py` whose body is a stub, assembles
  a snippet that is the ordered checklist, graded by checks for the key phrases
  (ThrottlingException step). `responses: []`.
- D2, a seeded failure the visitor runs: opens a `.py`, carries a `response`
  that is the failure itself (empty `retrievalResults`), so Run shows the
  failure. This is runnable, so it still needs a shell snippet — the
  control-plane call that checks whether the dependency is healthy, which is
  exactly step two of the diagnosis.

## Per-scenario authoring checklist

For each scenario in R4.18–R4.29:

1. Write the brief from the roadmap entry, all five fields, `breaksWhen` and
   `doneMeans` distinct and not restatements.
2. Author step 1: the diagram, nodes with icons, groups, labelled edges, no
   dangling edge.
3. For each row in the roadmap's step table, author a call step in pattern A,
   B, C, or D. Name the losing product and tie it to `breaksWhen`.
4. For every runnable step, add the shell terminal exercise and its authored
   success response.
5. For every exception claim, reproduce it against real botocore; correct or
   remove if it does not fire.
6. Register the document so `catalog()` loads it (the embed list in
   `gen-ai-core`), and confirm it parses.
7. Run the suite: the guards and tests should pass with no code change. If a
   test fails, the content is wrong, not the test — fix the content.

## Where the control-plane terminal commands come from

The terminal exercise for a runnable step is almost always a `get`/`list` on
the control-plane client for the service the step uses. A reference table to
author from (each has a simple scalar-flag form the shim handles and a plausible
success body to author):

- Knowledge-base retrieval → `bedrock-agent get-knowledge-base`,
  `list-knowledge-bases`, `list-data-sources`, `list-ingestion-jobs`.
- Agents → `bedrock-agent get-agent`, `list-agent-aliases`,
  `list-agent-action-groups`.
- Guardrails → `bedrock get-guardrail`, `list-guardrails`.
- Batch/custom models → `bedrock list-model-customization-jobs`,
  `get-model-invocation-job`.

Keep the authored response small and realistic (a status field, an id, a
timestamp), enough to show the command worked.

## Order of work

Author the scenarios in rough order of mechanism reuse, so the earliest ones
exercise the most-shared patterns and surface any authoring friction before the
long ones:

1. Case graph (6 steps) and Employee answers (5) — short, mostly pattern A.
2. Contract desk (9) and Product catalog search (10) — retrieval variants.
3. Member services (13), Benefits agent (9), Desk router (8) — agents.
4. Overnight claims (10), Support chat (14), Clinical intake (17) — the long
   operational and safety builds.
5. Brand voice (13) and Analyst workspace — training and the Quick build.

Each scenario is independent content; they can also be parallelized across
sessions since they share no code.

## Testing

- The existing `gen-ai-core` suite is the gate: every new document is loaded by
  `catalog()` and must pass every guard and test. No per-scenario test is
  needed beyond that, because the guards encode the methodology.
- `smoke-test.mjs` is extended to cover at least one runnable step per scenario
  that makes a distinct exception claim, per R4.15 — the python runs through
  the intercept and the claimed exception (or clean result) is asserted.
- A brief-completeness test already fails the build for a missing `breaksWhen`
  or `doneMeans`; keep it covering every new scenario.

## Risks

- **Content drift from the methodology.** Mitigated by the guards: a scenario
  that violates a rule fails `cargo test`, not review. The one unenforced rule
  (R4.3, prose names a losing product) stays a review item.
- **Unverified exception claims.** The phase 3 trap. Every claim is reproduced
  against botocore; the smoke test covers the ones that rest on a specific
  exception.
- **Terminal exercises that cannot succeed.** Some operations (like `retrieve`)
  need a structured body the one-flag shim cannot express. Author the terminal
  command as a `get`/`list` that the shim handles, never a data-plane call with
  a complex body. The reference table above lists safe choices.
- **Scope creep toward a new mechanism.** If a scenario seems to need one, it is
  almost certainly a pattern-D diagnosis step or a stated sandbox boundary, not
  a feature. Re-read the methodology before adding anything.
