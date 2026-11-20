# Phase 3.5 — Design

How the dual-pane step is built, reusing what exists. The guiding constraint:
add a second editor and point it at the Check mechanism already shipped. No new
execution engine, no new check semantics.

## What already exists (and is reused as-is)

- `CallStep` already has both `workbench.responses` (intercepted bodies, makes
  a step runnable) and `workbench.checks` (the `Check` answer key). Neither is
  new.
- `Check` (`pattern`, `present`, `message`) and the `runChecks()` method in
  `workbench.js` already implement required/forbidden substring verification
  with per-check messages. Reused verbatim for the infra editor.
- The code editor, Run, Pyodide boot, and the intercept are a self-contained
  path. Untouched.
- The language chip + reference-snippet marking already distinguishes python
  (assemble into the editor) from typescript (the CDK). Reused to route
  snippets to the right editor.
- `infrastructure: String` on `CallStep` holds the reference CDK shown in the
  Infrastructure pane. The infra *editor* seeds from an infra seed file and the
  typescript snippets, not from this read-only string — the string stays as the
  "here is the finished shape" reference, mirroring how the code path works.

## The model change (small)

Today the workbench payload sends `runnable` (from `is_runnable()`) and the top
-level `checks`, and `workbench.js` branches: runnable → code editor + Run +
CLI; else → one editor + Check. The dual step needs the JS to stop treating
these as mutually exclusive.

Introduce an explicit notion of the step's *shape* in the payload rather than a
single boolean:

```jsonc
{
  "workbench": { ... },
  "workerUrl": "...",
  // replaces the single `runnable` boolean at the decision points
  "panes": {
    "code":  true,   // has responses → code editor + Run + Pyodide + CLI
    "infra": true    // has checks   → infra editor + Check
  },
  "checks": [ ... ]  // unchanged; consumed by the infra pane now
}
```

`panes.code = !responses.is_empty()`, `panes.infra = !checks.is_empty()`. The
three combinations:

| code | infra | step kind |
| --- | --- | --- |
| true | false | pure runnable (today's runnable step) |
| false | true | pure config (today's config step) |
| true | true | **dual** (this subphase) |
| false | false | rejected at parse (`ConfigStepHasNoChecks`) |

`is_runnable()` stays as a helper (code pane present), so existing callers and
tests keep meaning. The renderer computes `panes` from the two vecs.

## Rust: `scenario.rs`

- Keep `is_runnable()` = `!responses.is_empty()`.
- Add `has_infra_check()` = `!checks.is_empty()` for symmetry and readability.
- Relax the guards:
  - `ConfigStepHasNoChecks` fires only when BOTH are empty (unchanged — already
    the condition).
  - `RunnableStepHasNoPython` fires when the code pane is present but no python
    snippet exists (unchanged).
  - Add: when the infra pane is present (checks non-empty), require at least
    one typescript snippet and an infra seed file, so the infra editor has
    something to assemble into and open. Call it `InfraCheckWithoutInfra` or
    fold into the existing snippet checks. Without this, a step could carry
    infra checks with nothing to type, the infra-side version of the
    worksheet-with-no-answer-key.
- The integrity test (`every_config_check_can_be_satisfied_by_assembling_the_snippets`)
  changes: match `present: true`/`false` infra checks against the **typescript**
  snippets' sources specifically, not all snippets. Today it concatenates all
  snippet sources; scope it to the language the infra editor assembles. (Code
  checks do not exist — the code pane is verified by running, not checks — so
  this is purely about infra checks vs typescript snippets.)

## Rust: `render.rs`

- `workbench_json`: emit `panes: { code, infra }` as above. Keep sending
  `checks`. Optionally keep `runnable` for one release for backward safety,
  then drop it once the JS reads `panes`.
- The workbench template region: today one editor region plus the console/CLI.
  Render a second editor region below the code editor when `panes.infra`. Give
  each region a heading ("Code" / "Infrastructure") and its own output pane.
  The infra region needs: a gutter-bearing textarea, a Check button, an output
  pane. No completion menu (not Python).
- A seed file for the infra editor: the workbench `files` already carries seed
  files keyed by path; the infra editor opens a `.ts` seed (e.g. `stack.ts`)
  the same way the code editor opens its `.py`. A dual step seeds both files.
  `workbench.open` names the code file; add a convention for which file the
  infra editor opens — simplest is a second field, `openInfra`, or a naming
  convention (the `.ts` file in `files`). Prefer an explicit `openInfra:
  Option<String>` on `Workbench` to avoid guessing by extension.

## Browser: `workbench.js`

- The constructor/mount reads `panes` instead of the single `runnable`.
- Factor the editor construction into a small helper that builds a
  gutter+textarea+output triple for a given seed file, so both editors share
  it. The code editor additionally wires Pyodide completion and Run/Stop/CLI;
  the infra editor wires only Check.
- `runChecks()` is reused unchanged, but parameterized by which buffer it reads
  (the infra textarea) and which output pane it writes. Today it reads
  `this.textarea`; generalize to take the infra editor's textarea and output.
- The code editor keeps `this.textarea`/`this.output`/`boot()`; the infra
  editor gets its own `this.infraTextarea`/`this.infraOutput`. Keep them
  distinct so a Run result and a Check result never overwrite each other.
- Step-swap (`resync`) seeds both editors and both check sets, extending the
  existing resync so a Next between dual steps does not leave a stale infra
  buffer or answer key (the same class of bug the nav resync already guards).

## CSS: `styles.css`

- Stack the two editors vertically in the workbench column, each with its
  heading, textarea, button row, and output. Reuse the existing `.lab-code` /
  editor styles; add a divider or spacing token between them so it reads as two
  tools, not one scrolling box.
- On a narrow viewport they remain stacked (they already are); nothing special.

## Content: making a step dual

A dual step in `policy-assistant.json` (or a phase-4 scenario) carries:

- `snippets`: python snippets (assemble into the code editor) **and**
  typescript snippets (assemble into the infra editor), each language-tagged.
- `workbench.files`: a `.py` seed (code) and a `.ts` seed (infra), with
  `open` naming the `.py` and `openInfra` naming the `.ts`.
- `workbench.responses`: the intercepted bodies for the code call.
- `workbench.checks`: the infra answer key (required CDK constructs, forbidden
  anti-patterns), matched against the typescript snippets.

Worked example candidate: the "Retrieve, then Converse" step already has both a
real call and a stack behind it, or a phase-4 step authored dual from the
start. Pick one; do not convert all steps.

## Testing

- Unit (`lib.rs`): a dual step parses; the integrity test matches its infra
  checks against its typescript snippets; the guards reject a step with infra
  checks but no typescript snippet / no infra seed.
- Unit (`render.rs`): the rendered dual step contains two editor regions, two
  buttons (Run and Check), and both seed files; a pure step contains one.
- Smoke (`smoke-test.mjs`): the dual step's python assembles and runs through
  the intercept (the existing runnable-step oracle already covers this once the
  step has responses). The infra checks are browser-side, so their
  satisfiability is covered by the Rust integrity test, not the smoke test.
- Browser verify: two editors render stacked; Run prints the call result; Check
  reports infra pass/fail independently; a Next to another step reseeds both.

## Risks and mitigations

- **Payload/JS coupling.** Changing `runnable` → `panes` touches render.rs and
  workbench.js together. Mitigate by keeping `runnable` in the payload for the
  transition and having the JS prefer `panes` when present, then removing
  `runnable` in a follow-up once verified.
- **Two output panes confusing the smoke test's extraction.** The smoke test
  reads the code path only; it does not touch the infra editor. Keep the infra
  editor's DOM ids distinct so nothing the harness scrapes changes.
- **Resync staleness.** The existing nav resync bug (workbench holding the
  previous step's data) now has a second buffer and answer key to reset. The
  resync path must seed both or a dual→dual Next grades against the wrong infra.
  Explicit task.
- **Scope creep toward execution.** The temptation to "just typecheck the TS"
  is Level 3 from the estimate and is out of scope. The infra editor is
  structural-check only; this is stated so a later contributor does not quietly
  add a compiler.

## Amendment — the CLI pane

The CLI pane reuses two things that already exist: the `cliToPython` shim (argv
→ a boto3 call against the intercepted client) and the `Check` model. It adds a
third editor-like pane and a verify path that is the shim run followed by the
checks.

### Model (scenario.rs)

- New `CliPane` struct on `Workbench` as `cli: Option<CliPane>`,
  `skip_serializing_if = "Option::is_none"`:
  ```rust
  pub struct CliPane {
      /// Placeholder shown in the empty terminal input, e.g.
      /// "bedrock-agent-runtime retrieve --knowledge-base-id KB123 …".
      pub prompt: String,
      /// The answer key for the command line. Reuses Check: required flag
      /// present, forbidden anti-pattern absent, message shown either way.
      pub checks: Vec<Check>,
  }
  ```
- `has_cli(&self) -> bool = self.workbench.cli.is_some()` on `CallStep`.
- Guards:
  - `CliPaneWithoutCall`: `cli.is_some()` but `!is_runnable()` → reject. The
    shim needs an intercept.
  - A `cli` block whose `checks` is empty → reject (fold into the
    `CliPaneWithoutCall` message set or a dedicated `CliPaneHasNoChecks`).
- The CLI checks are the command line's answer key, not the snippets'. The
  existing integrity test matches infra checks against typescript snippets and
  must NOT pull CLI checks into that set — CLI checks are verified by running
  the command in the browser, so they get no snippet-reachability test (the
  same reason code steps get none). Scope the integrity test to
  `workbench.checks` only, leaving `cli.checks` out of it.

### Payload (render.rs)

- `workbench_json`: `panes` becomes `{ code, cli, infra }` with
  `cli: call.has_cli()`. Keep emitting the top-level `checks` (infra). The
  `cli` block already serializes inside `workbench`, so the JS reads
  `data.workbench.cli` for the prompt and the CLI checks.
- The template drops the CLI out of the code pane's slot and gives it its own
  region between `.lab-editor`/`.lab-output` and `.lab-editor-infra`. Rename
  the existing `.lab-cli` usage: it becomes the standalone CLI pane container
  with its own output `.lab-cli-output`. The code pane no longer owns a CLI.

### Browser (workbench.js)

- `readPanes` reads the third member; fall back keeps `cli: false` for a legacy
  block.
- The CLI form moves out of the code editor's render branch into its own
  `panes.cli` branch: build the `aws` prompt + input in `.lab-cli`, seed the
  placeholder from `data.workbench.cli.prompt`, and wire submit to a new
  `verifyCli(line)`.
- `verifyCli` replaces the old `runCli`'s "dump JSON into code output"
  behavior:
  1. Translate with `cliToPython`. A null (malformed argv) is an immediate
     fail in the CLI output, no worker round-trip.
  2. Run the snippet through the worker exactly as before, but tag the run so
     its result returns to the CLI output, not the code output. A traceback
     (ParamValidationError, UnknownOperation) is a fail with the exception
     shown. A clean run is a pass on the "botocore accepted it" half.
  3. On a clean run, apply `cli.checks` to the raw command line with the same
     `runChecks` logic, generalized to take (text, checks, output). Print the
     per-check verdict below the "command accepted" line.
- Keep the code Run path untouched. The worker already distinguishes runs by
  the file path it writes; the CLI uses `__cli__.py`, so route a result from
  that path to `this.cliOutput`. Track which pane a run belongs to
  (`this.pendingRunPane`) so `onMessage`'s `output`/`traceback` land in the
  right place.
- `resync` resets the CLI buffer/output alongside the others.

### CSS (styles.css)

- The `.lab-cli` form styling already reads as a terminal line; keep it. Add
  `.lab-cli-output` matching `.lab-output`/`.lab-output-infra`. Stack the three
  panes with the existing divider/spacing token so code, CLI, and infra read as
  three tools in a column.

### Content

- The worked dual step ("Retrieve, then Converse") gains a `cli` block: the
  prompt seeds the `retrieve` command, and the checks assert the call is
  `bedrock-agent-runtime retrieve` with `--knowledge-base-id`, and forbid
  sending Retrieve to the wrong plane. This makes the one step exercise all
  three tools: Run the Python, verify the CLI, Check the stack.

### Testing

- `scenario.rs`: a step with a `cli` block parses; `CliPaneWithoutCall` rejects
  a CLI block on a non-runnable step and one with no checks; the integrity test
  stays scoped to infra `checks` and ignores `cli.checks`.
- `render.rs`: a step with a CLI block emits `panes.cli = true` and renders the
  CLI region; a step without does not.
- `smoke-test.mjs`: unchanged — it scrapes the code path. The CLI verify is
  browser-side (worker + checks) like the infra Check, covered by browser
  verify, not the smoke oracle.
- `node --check assets/gen-ai/workbench.js`.

## Amendment 2 — supersedes the dual-editor and CLI-pane design above

Shipped design. Where it conflicts with anything above, this wins.

### Model (scenario.rs)

- `Workbench`: `files`, `open`, `responses`, `reasons`, `checks`. No
  `open_infra`, no `cli`, no `CliPane`.
- `CallStep::is_runnable()` = `!responses.is_empty()` — the one place the
  runnable/config distinction is read, and what decides whether the terminal
  renders. `has_checks()` = `!checks.is_empty()`.
- Guards: a step with neither responses nor checks still fails
  (`ConfigStepHasNoChecks`); a runnable step opening a `.py` with no python
  snippet still fails (`RunnableStepHasNoPython`). The infra/CLI guards
  (`InfraCheckWithoutInfra`, `InfraEditorOnNonDualStep`, `CliPaneWithoutCall`,
  `CliPaneHasNoChecks`) are gone with the fields they guarded.
- The integrity test matches `checks` against the typescript snippet sources
  (the `.ts` file is what Check grades), unchanged in spirit from before.

### Payload (render.rs)

- `workbench_json` emits `workbench` + `workerUrl` + `runnable`
  (`is_runnable()`) + `checks`. No `panes` map — one editor needs no pane
  enumeration; `runnable` alone decides the terminal.
- Template: one `.lab-editor`, one `.lab-output`, one `.lab-cli` +
  `.lab-cli-output` for the terminal. No `.lab-editor-infra` /
  `.lab-output-infra`.

### Browser (workbench.js)

- One editor, one buffer `Map` holding every seeded file. The file tree lists
  all files; `open(path)` loads the buffer, saves the previous one, and calls
  `updateAction(path)`.
- `updateAction` sets the single action button from the extension:
  `isRunnablePath` (`.py`) → "Run" (disabled on a non-runnable step);
  `isCheckPath` (`.ts`) → "Check"; otherwise disabled reference.
- `action()` dispatches: Run writes the buffer and runs it against the
  intercept; Check runs `runChecks(buffer, checks, output)`.
- The terminal renders under `this.runnable`. `runCommand(line)` →
  `cliToPython(line)`; a null (cdk / malformed) prints the honest message; an
  `aws` translation runs via the worker, tagged `pendingRunPane = 'terminal'`
  so `onMessage` routes the result to `finishCommand`/`showCommandOutput` in
  the terminal pane, leaving the editor output untouched.
- `cliToPython` returns null for a leading `cdk` and for argv too short or
  missing a flag, so the terminal never translates `cdk deploy` into a nonsense
  boto3 call.
- `resync` reseeds the one editor and the terminal on a step swap.

### CSS (styles.css)

- One editor, one output, the terminal (`$` prompt + input) and its output.
  No infra-editor divider/heading.

### Content

- The former "dual" step keeps its `.py` and `.ts` files in `files` with `open`
  on the `.py`, its `responses`, and its `checks`. No `openInfra`, no `cli`
  block. Opening the `.ts` in the one editor gives Check; the terminal runs the
  `retrieve` call as an `aws` command.

### Testing

- `scenario.rs`: a both-languages step parses and is runnable; a pure config
  step parses and is not; the teaches-nothing step still fails.
- `render.rs`: the payload carries both files and the checks; the terminal
  renders only on a runnable step; no `lab-editor-infra`/`lab-output-infra` in
  the HTML.
- `node --check assets/gen-ai/workbench.js`.
