# Phase 3.5 — Dual-pane step: run the code and check the infrastructure

A subphase, small and self-contained, slotted between phase 3 (one scenario,
authored) and phase 4 (the catalog). It makes a single step able to teach both
halves of a decision at once: the Python call you Run against the intercept,
and the CDK infrastructure you assemble and Check against an answer key, in two
editors stacked in the workbench.

Today a step is one or the other. A runnable step has a code editor, a Run
button, Pyodide, and intercepted responses. A configuration step has one
editor holding a CDK/config artifact and a Check button that runs structural
substring checks (the answer key), no execution. This subphase lets a runnable
step *also* carry an infrastructure editor with its own Check, so the visitor
practices the call and the stack that provisions it in the same step.

## Why this is the right ceiling

The teaching model stays honest and unchanged in kind:

- Code is **executed** against real vendored botocore, because a sandbox can
  genuinely verify a call's shape and the library's own validation.
- Infrastructure is **structurally checked** against required/forbidden
  patterns, because a browser sandbox cannot deploy CDK and a faked synth would
  be dishonest. This is already the decided ceiling for config steps; this
  subphase reuses it rather than inventing a parallel mechanism.

No in-browser CDK synth, no TypeScript compiler, no CloudFormation emulation.
Those were considered and rejected in design as disproportionate.

## Goals

1. A single call step can present two editors stacked in the workbench: the
   code editor (Run) above, an infrastructure editor (Check) below.
2. The infrastructure editor is seeded as a scaffold, assembled from the step's
   infrastructure snippets, and checked by the same structural-check mechanism
   config steps already use.
3. Existing pure-runnable steps (code only) and pure-config steps (infra only)
   keep working exactly as they do. The dual step is additive.
4. The feature is authored per step: a step opts in by carrying both
   intercepted responses (for the code) and infrastructure checks (for the
   infra). Nothing becomes dual automatically.

## Non-goals

- No execution of the infrastructure. No synth, no deploy, no typecheck of CDK.
- No new check semantics. The infra editor reuses the existing `Check` model
  (required/forbidden substring + message) verbatim.
- No change to the code/Run/Pyodide/intercept path. It is untouched; the infra
  editor sits beside it.
- No requirement that every step become dual. Phase 4 scenarios opt in where a
  decision genuinely has both a call and a stack worth practicing.

## Functional requirements

- R3.5.1 A `CallStep` may simultaneously have intercepted `responses` (making
  it runnable) and `checks` (an infrastructure answer key). The current model
  already has both fields; today the parser treats them as mutually relevant
  via `is_runnable()`. This subphase makes "both present" a valid, meaningful
  combination.
- R3.5.2 When a step has both, the workbench renders two editors: the code
  editor bound to the code seed file (Run), and a second editor bound to the
  infrastructure seed (Check). Both are real `<textarea>` editors with the same
  gutter/completion affordances the code editor has, except the infra editor
  needs no Pyodide completion (it is not Python).
- R3.5.3 The infrastructure editor is a scaffold assembled from the step's
  infrastructure snippets, consistent with how the code editor is a scaffold
  assembled from the code snippets. The visitor writes the stack; the step does
  not ship it finished.
- R3.5.4 The infra Check runs the step's `checks` against the infra editor's
  buffer using the exact `runChecks` logic (required pattern present, forbidden
  pattern absent, message shown pass or fail). Output appears in a pane
  associated with the infra editor, distinct from the code Run output.
- R3.5.5 A pure-runnable step (responses, no checks) renders only the code
  editor, as today. A pure-config step (checks, no responses) renders only the
  infra editor with Check, as today. Only the both-present case renders two.
- R3.5.6 The infra snippets in a dual step are labelled with their language
  (typescript), consistent with the language-chip work: the code snippets are
  python and assemble into the code editor; the infra snippets are typescript
  and assemble into the infra editor. The existing reference-snippet marking
  still applies.

## Validation and guards

- R3.5.7 The parse-time guards from the earlier work are relaxed to permit the
  dual case without losing their protection:
  - `ConfigStepHasNoChecks` (a step with no responses AND no checks) still
    fires — a step that teaches nothing still fails.
  - `RunnableStepHasNoPython` still fires for a runnable step whose code editor
    would have no Python to assemble.
  - A new rule or an adjustment ensures a *dual* step is well-formed: it has at
    least one python snippet (for the code editor) and at least one typescript
    snippet (for the infra editor), an infra seed file, and at least one check.
- R3.5.8 The existing answer-key integrity test (every `present: true` check
  pattern appears in the assemblable snippets; every `present: false` pattern
  does not) extends to dual steps, matching infra checks against the infra
  (typescript) snippets specifically, not the python ones.

## Accessibility

- R3.5.9 Two editors on one step each have a clear label ("Code" / "Run" and
  "Infrastructure" / "Check") so a screen-reader user knows which is which and
  which button acts on which buffer. Headings or `aria-label`s, not visual
  proximity alone.
- R3.5.10 Both editors are keyboard operable with visible focus, and each
  Check/Run result is announced in a region associated with its editor.

## Acceptance

- A dual step renders two stacked editors; Run executes the code against the
  intercept and prints its result; Check verifies the infra buffer against the
  answer key and reports per-check pass/fail.
- Pure-runnable and pure-config steps are visually and behaviorally unchanged.
- `scenario.rs` guards accept the dual case and still reject the degenerate
  ones; the integrity test covers infra checks against infra snippets.
- The smoke test covers a dual step: its code runs through the intercept, and
  (in a unit test, since checks are browser-side) its infra checks are
  satisfiable by its infra snippets.
- One real step in the policy-assistant scenario is converted to dual as the
  worked example, or a phase-4 step is authored dual, demonstrating the feature
  end to end.

## Amendment — the CLI pane (third tool)

Added after the dual-pane work landed. The dual step settled "infrastructure"
as TypeScript/CDK assembled into an editor and structurally Checked — it is not
CLI commands. But practicing the AWS CLI is still worth teaching, so it becomes
its own tool rather than being folded into either the code editor or the infra
editor.

Today a one-line `aws …` input already sits inside the code pane and routes
through the `cliToPython` shim to the same intercepted client, dumping JSON
into the code output. This amendment promotes that from a line tucked under the
code editor into a first-class, verified pane with its own verdict, stacked
between the code editor and the infra editor.

### What the CLI pane is

- A single-line terminal: an `aws` prompt and one input, styled to read as a
  real shell line, not a form field. One command per submit (not a multi-line
  script).
- It does not need real CLI output. It needs to verify the command is shaped
  right before the visitor moves on. "Shaped right" is checked two ways, both
  honest:
  1. The command runs through the existing `cliToPython` shim against the real
     intercepted botocore client. A wrong operation, a missing required
     parameter, or a malformed argument fails for real — the library's own
     validation, the same honesty the code pane relies on.
  2. On a call that botocore accepts, the step's authored CLI `checks` run
     against the normalized command line (required flag present, forbidden
     anti-pattern absent, message shown), the same `Check` model the infra
     editor uses.
- A verdict is shown: the call succeeded or failed, then the per-check
  pass/fail with its message. The visitor knows the command is right before
  continuing.

### Functional requirements

- R3.5.11 A `CallStep` may carry an optional `cli` block. When present, the
  workbench renders a CLI pane. When absent, no CLI pane renders — opt-in per
  step, exactly like the infra pane.
- R3.5.12 `panes` gains a third member: `{ code, cli, infra }`. `panes.cli` is
  true when the step authors a `cli` block. The three panes stack in a fixed
  order: code editor (Run), CLI terminal (verify), infra editor (Check).
- R3.5.13 The CLI pane requires a runnable step: the shim submits to the
  intercept, so a step with a CLI block but no `responses` has nothing to
  validate against and fails parse. CLI is a companion to the code pane, not an
  alternative to it.
- R3.5.14 The CLI `checks` reuse the existing `Check` type (pattern, present,
  message) verbatim, matched as substrings against the normalized command
  line, not against snippets.
- R3.5.15 The CLI verdict is distinct from the code Run output and the infra
  Check output. A Run, a CLI verify, and an infra Check never overwrite each
  other's result.

### Validation and guards

- R3.5.16 A `cli` block on a non-runnable step is rejected
  (`CliPaneWithoutCall`): there is no intercept to validate the command
  against.
- R3.5.17 A `cli` block with no checks is rejected: a terminal that verifies
  nothing teaches nothing, the same rule config/infra already follow.

### Accessibility

- R3.5.18 The CLI pane has its own label ("Command line" / "Run") so a screen
  reader distinguishes it from the code editor's Run and the infra editor's
  Check. Its verdict is announced in a region associated with the pane.

## Amendment 2 — supersedes the dual-editor and CLI-pane amendments above

The two amendments above (the two-editor "dual" step, and the authored CLI
pane with a per-step answer key) were built, then reconsidered and replaced.
This amendment is the shipped design; where it conflicts with anything above,
this wins. The earlier text is kept only as a record of the path taken.

Why the change:

- Two stacked editors (code + infra) was more apparatus than the teaching
  needed. The visitor does not need two editors to practice a call and its
  stack; one editor that holds every file and switches between them is simpler
  and reads better.
- The authored CLI pane had no lesson behind it: no step teaches a CLI command,
  `cdk` cannot execute in a no-cloud sandbox, and the only honest verification
  (`aws` → botocore) is the same one the Python editor already gives. A pane
  with a seeded placeholder and an answer key for content that does not exist
  was scaffolding pretending to be a feature.

The shipped design:

- R3.5.1′ One editor. It tracks every seeded file — `.py`, `.ts`, and
  reference files like `NOTES.md` — and switches between them from the file
  tree. There is no second editor and no file held out of the tree, so there
  is no pair of buffers that can fight over one file (the bug the two-editor
  design spent most of its code guarding against).
- R3.5.2′ The action button follows the open file's extension. A `.py` file
  Runs against the intercept; a `.ts` file is Checked against the step's
  `checks`; a file that is neither (notes, data) is readable reference with the
  button disabled. One button, two jobs, chosen by what is on screen.
- R3.5.3′ A step carries `responses` (making its `.py` runnable), `checks`
  (making its `.ts` checkable), or both, in the one editor. `open` names the
  starting file. There is no `openInfra`, no `panes`, no `cli` block, no
  `CliPane`.
- R3.5.4′ A terminal sits below the editor on every runnable step (one with
  responses, so an intercept exists to submit to). It is a plain `$` prompt
  with a neutral placeholder and no seeded value. The visitor types a full
  command.
- R3.5.5′ The terminal just runs. An `aws` command is translated to a boto3
  call and executed against the same intercept the editor uses; botocore's
  acceptance (the response prints) or rejection (a real `ParamValidationError`,
  unknown-operation, etc. prints) is the whole verification. There is no
  authored answer key for the terminal.
- R3.5.6′ A command the sandbox cannot run — a `cdk` line, or malformed argv —
  prints a plain message saying so (`aws` runs here, `cdk` has no cloud to
  deploy to) rather than failing silently or faking success.
- R3.5.7′ The terminal result and the editor Run/Check result write to
  separate output panes and never clobber each other.

Acceptance (replaces the dual-step acceptance above):

- One editor shows every seeded file; opening a `.py` gives Run, a `.ts` gives
  Check, a reference file disables the button.
- A runnable step shows the terminal; a pure config step does not.
- An `aws` command runs and prints its intercepted result or its real botocore
  error; a `cdk` command prints the honest "can't run here" message.
- `scenario.rs` guards accept a both-languages step and still reject a step
  that teaches nothing; no `openInfra`/`cli`/`panes` remain.
