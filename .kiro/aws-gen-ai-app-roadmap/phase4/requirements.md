# Phase 4 — The rest of the catalog, at the policy-assistant grain

Status: not started

Goal: author every remaining scenario in the catalog to the same shape phase 3
built and phase 3.5 refined, so the union of the scenarios is the Generative AI
Developer exam. Phase 3 built one scenario (the EU policy assistant) end to
end. Phase 3.5 settled the workbench: one editor, an always-on terminal, honest
verification. Phase 4 is content — the other scenarios, each authored to the
methodology below, with the parse guards and tests already enforcing most of
it.

No new mechanism. If a scenario seems to need one, that is a signal to
re-read the methodology, not to build a feature. The one allowed exception is a
guard or test that makes an authoring rule enforceable instead of
hand-followed.

## The methodology every scenario follows

This is the settled shape, distilled from the policy-assistant build and its
audit. Each rule is already enforced by a parse guard or a `gen-ai-core` test
unless marked "(review)".

### Step shape

- R4.1 Step 1 is the architecture diagram, full width, no workbench. Company
  and system in the job's terms, AWS Architecture Icons over product names,
  dashed Region/VPC groups, labelled orthogonal connectors. (The diagram
  guards from phase 3 apply: no dangling edge, geometry in the hashed style
  block.)
- R4.2 Every later step is one `call` step: a decision, a why, labelled
  snippets to assemble, an infrastructure block, a seeded workbench.
- R4.3 Every call step names the product that loses and why losing it walks
  toward the scenario's `Breaks when`. A step whose prose is only "use service
  X" fails review. (review)

### The single editor

- R4.4 Each step seeds one editor with every file it needs — `.py`, `.ts`, and
  reference files (`NOTES.md`, data). The file tree switches between them; the
  action button follows the open file's extension: `.py` Runs against the
  intercept, `.ts` is Checked against the answer key, a reference file is
  read-only.
- R4.5 `open` names the starting file. There is no second editor, no
  `openInfra`, no `cli` block, no `panes` — those were removed in 3.5.

### Runnable vs configuration

- R4.6 A step is runnable when it carries `responses` (intercepted bodies for
  the calls its snippets make). A runnable step opens a `.py`, assembles python
  snippets, and teaches by running real botocore.
- R4.7 A configuration step carries `checks` and no `responses`. It opens a
  `.ts` (or other config file), assembles typescript snippets, and is graded by
  the structural answer key. Checks come in present/absent pairs: the absent
  checks carry the "no wildcard IAM, no fine-tune job" half of the lesson.
- R4.8 A step with neither `responses` nor `checks` fails to parse
  (`ConfigStepHasNoChecks`). Every step teaches something verifiable.

### The terminal exercise (the 3.5 + audit rule)

- R4.9 Every runnable step shows the always-on terminal, so every runnable step
  carries at least one `shell` snippet teaching the `aws` command to type into
  it. A runnable step with no shell snippet fails to parse
  (`RunnableStepHasNoShell`). This is the rule the audit added: the terminal is
  never a tool with no lesson.
- R4.10 The terminal exercise is a success path. The shell snippet's operation
  is answered by an authored `response` so the command genuinely runs and
  prints a result — not a command engineered to fail. A `get-*`/`list-*`
  control-plane call is the natural fit: it verifies the thing the step's data
  plane call depends on, and it reinforces the control-plane/data-plane split.
- R4.11 The terminal just runs: `aws` commands execute against the same
  intercept the editor uses, botocore validates them for real, and a `cdk` or
  non-`aws` line prints the honest "can't run here" message. No authored answer
  key for the terminal; botocore's acceptance is the verification.
- R4.12 Every intercepted `response` is reachable: its operation appears in a
  snippet, as a python method (`.op(`) or the CLI kebab form (`aws svc op-name`).
  An unreachable response fails
  (`seeded_code_only_calls_operations_the_step_answers`).

### Snippet languages

- R4.13 Snippets are `python`, `typescript`, or `shell`. Python assembles into
  the `.py` editor and runs; typescript assembles into the `.ts` editor and is
  checked; shell is the terminal exercise. Any other language fails
  (`UnknownSnippetLanguage`). A runnable step opening a `.py` needs at least one
  python snippet (`RunnableStepHasNoPython`).
- R4.14 The scaffold is not the answer: the open file is a stub the visitor
  fills from the snippets, never the finished call
  (`the_seed_is_a_scaffold_not_the_answer`).

### Content accuracy (carried from phase 3, still the hard constraint)

- R4.15 Every exception a step claims — in a `reason`, in NOTES, or in prose —
  is reproduced against the real vendored botocore before the step ships. If it
  does not fire, the step is corrected to what does, or the claim is removed.
- R4.16 A step that depends on server-side validation the intercept cannot
  reach (residency prefixes, quota, throttling, IAM denials) teaches that as a
  stated sandbox boundary in NOTES, or is reframed to a client-side-checkable
  mistake. It is never faked.
- R4.17 Diagnosis steps (no live call possible — throttling, an IAM denial, a
  cost estimate) are configuration steps whose "artifact" is an ordered
  checklist in a `.md` or `.py`, graded by checks for the key phrases. The
  policy assistant's ThrottlingException and empty-retrieval steps are the
  templates.

## The scenarios to build

Each is a catalog entry with a brief (`whoIsAsking`, `whatTheyNeedBack`,
`systemsAlreadyRunning`, `breaksWhen`, `doneMeans`) and a step table already in
the roadmap. Build each to the methodology above.

- R4.18 Product catalog search — hybrid search on an existing OpenSearch
  cluster; the receipt-lookup failure is the `Breaks when`.
- R4.19 Contract desk — pgvector on an existing Aurora; cross-client quotation
  is the `Breaks when`.
- R4.20 Case graph — Neptune Analytics graph retrieval; an undefendable
  asserted edge is the `Breaks when`.
- R4.21 Employee answers — Amazon Q Business over connected sources with their
  own ACLs; a permissions leak is the `Breaks when`.
- R4.22 Member services — a new AgentCore agent with return-of-control on
  refunds; the model moving money is the `Breaks when`.
- R4.23 Benefits agent — a live classic Bedrock Agent changed without pointing
  production at a draft; production serving a draft is the `Breaks when`.
- R4.24 Desk router — multi-agent routing with tool isolation; a billing turn
  reaching clinical tools is the `Breaks when`.
- R4.25 Overnight claims — batch inference, no caller waiting; one corrupt
  packet failing the night is the `Breaks when`.
- R4.26 Support chat — streaming, prompt caching, latency; a cached answer
  carrying another caller's balance is the `Breaks when`.
- R4.27 Clinical intake — guardrails, PII, auditability; the identifier
  reaching screen/log/chart is the `Breaks when`.
- R4.28 Brand voice — fine-tuning for voice, retrieval for facts; stale copy in
  the brand's voice is the `Breaks when`.
- R4.29 Analyst workspace (Amazon Quick) — the confirmed Quick build; authored
  to the same grain. (Step table to be drawn from the roadmap's Quick note.)

## Constraints carried forward

- R4.30 `gen-ai-core` still has no `aws-sdk-*` or `lambda_*`; scenarios stay
  content embedded at build time. One HTTP binary serves them.
- R4.31 No step is addressable; Next is a posted form value and the workbench
  survives the swap. No routing or session-model change.
- R4.32 Everything the methodology requires is enforced by a guard or a test,
  so a scenario that violates it fails the build rather than reaching a visitor.

## Exit criteria

Every catalog scenario walks from its diagram through its final failure step,
each step a real decision with a losing product named. Every runnable step runs
its python against the intercept and offers a working terminal command. Every
configuration step is graded by present/absent checks. Every exception claim is
verified against real botocore. The whole catalog parses under the phase 3/3.5
guards with no new mechanism, and the landing lists every built scenario. None
of it costs a Lambda invocation beyond serving the page.
