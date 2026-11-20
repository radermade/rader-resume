# Phase 3.5 — Tasks

Rendering + workbench + one authored step. Reuses the existing Check mechanism;
adds no execution engine. Work top to bottom.

## Investigate first

- [ ] Re-read `workbench.js` mount/`resync` and `runChecks` to confirm the
      exact `this.textarea` / `this.output` coupling the infra editor must
      parameterize around.
- [ ] Re-read `render.rs` `workbench_json` and the workbench template region to
      see where a second editor region and the `panes` payload slot in.
- [ ] Confirm `Workbench` fields in `scenario.rs` (`files`, `open`,
      `responses`, `reasons`, `checks`) before adding `openInfra`.

## Model (scenario.rs)

- [ ] Add `openInfra: Option<String>` to `Workbench` (which seed file the infra
      editor opens). `skip_serializing_if = "Option::is_none"`.
- [ ] Add `has_infra_check(&self) -> bool` = `!workbench.checks.is_empty()` on
      `CallStep`, beside `is_runnable()`.
- [ ] Relax/extend parse guards:
  - [ ] `ConfigStepHasNoChecks` unchanged (both empty still fails).
  - [ ] `RunnableStepHasNoPython` unchanged.
  - [ ] New guard: if the infra pane is present (checks non-empty), require at
        least one `typescript` snippet and that `openInfra` names a seeded
        file. Error variant e.g. `InfraCheckWithoutInfra`.
- [ ] Extend the integrity test so infra checks are matched against the
      concatenation of **typescript** snippet sources (not all snippets).

## Payload (render.rs)

- [ ] `workbench_json`: emit `panes: { code: is_runnable(), infra:
      has_infra_check() }`. Keep `checks`. Keep `runnable` for the transition.
- [ ] Render a second editor region below the code editor when `panes.infra`:
      heading "Infrastructure", gutter+textarea seeded from `openInfra`, a
      Check button, an output pane with distinct DOM ids.
- [ ] Give the code region a clear "Code"/"Run" heading so the two read as
      distinct tools.
- [ ] Pure-runnable and pure-config render exactly one editor (regression).

## Browser (workbench.js)

- [ ] Read `panes` (prefer it; fall back to `runnable` if `panes` absent).
- [ ] Factor editor construction into a helper building
      gutter+textarea+output for a given seed file.
- [ ] Code editor: keep Pyodide completion, Run/Stop, CLI, `boot()`.
- [ ] Infra editor: textarea + Check only; no completion, no Pyodide.
- [ ] Generalize `runChecks` to take (buffer, outputPane) so it serves the
      infra editor; keep behavior identical.
- [ ] Keep `this.textarea/output` (code) and `this.infraTextarea/infraOutput`
      distinct so Run and Check results never clobber each other.
- [ ] Extend `resync` to reseed both editors and both check sets on a step
      swap, so a dual→dual Next does not grade against stale infra.

## Styling (styles.css)

- [ ] Stack the two editors vertically with a divider/spacing token; reuse
      `.lab-code` and editor styles. Visible focus on both; Check/Run buttons
      share the interactive-accent treatment.

## Content — one worked dual step

- [ ] Pick one step to make dual (candidate: "Retrieve, then Converse", which
      has both a real call and a stack) or author a phase-4 step dual.
- [ ] Add typescript infra snippets and a `.ts` infra seed; set `openInfra`.
- [ ] Add infra `checks` (required constructs, forbidden anti-patterns) matched
      to the typescript snippets.
- [ ] Keep the python snippets, `.py` seed, and `responses` for the code pane.
- [ ] Do NOT convert every step; dual is opt-in per step.

## Tests

- [ ] `scenario.rs`: dual step parses; new guard rejects infra-checks-without-
      typescript-snippet / missing `openInfra` target; integrity test scopes
      infra checks to typescript snippets.
- [ ] `render.rs`: dual step renders two editor regions, Run and Check buttons,
      both seed files; pure steps render one.
- [ ] `smoke-test.mjs`: the dual step's python still runs through the intercept
      (existing runnable oracle). Infra checks are covered by the Rust
      integrity test, not the smoke test.
- [ ] `cargo test` green; `cargo clippy --all-targets --all-features -- -D
      warnings` clean; `cargo fmt --check` clean; `node --check
      assets/gen-ai/workbench.js`.

## Browser verify (manual, recorded)

- [ ] Dual step shows two stacked editors, each labelled, each with its button.
- [ ] Run executes the code and prints the intercepted result in the code
      output; Check verifies the infra buffer and prints per-check pass/fail in
      the infra output; the two do not interfere.
- [ ] A Next from one dual step to another reseeds both editors and both answer
      keys (no stale infra).
- [ ] Keyboard: both editors and both buttons reachable with visible focus.
- [ ] Pure-runnable and pure-config steps look and behave as before.

## Follow-up (not this subphase)

- [ ] Remove the transitional `runnable` payload field once `panes` is verified
      in the browser.

## Out of scope (stated so it is not quietly added)

- In-browser CDK synth, deploy, or CloudFormation emulation.
- Any TypeScript compilation or typecheck of the infra editor's contents. The
  infra editor is structural-check only, the same honest ceiling config steps
  already use.

## Amendment — the CLI pane (third tool)

A one-line terminal between the code editor and the infra editor. Verifies the
command through the real intercept (botocore validates) then authored checks on
the command line. Opt-in per step via a `cli` block. Reuses `cliToPython` and
`Check`; adds no new execution.

### Model (scenario.rs)

- [ ] Add `CliPane { prompt: String, checks: Vec<Check> }` and
      `cli: Option<CliPane>` on `Workbench` (`skip_serializing_if`).
- [ ] Add `has_cli(&self) -> bool` on `CallStep`.
- [ ] Guard `CliPaneWithoutCall`: `cli.is_some()` on a non-runnable step →
      reject. Also reject a `cli` block with empty checks.
- [ ] Keep the integrity test scoped to `workbench.checks`; CLI checks are
      browser-verified, not snippet-reachable.

### Payload + template (render.rs)

- [ ] `panes` → `{ code, cli, infra }`, `cli: call.has_cli()`.
- [ ] Give the CLI its own region between code output and the infra editor; add
      `.lab-cli-output`. Remove the CLI from inside the code pane.

### Browser (workbench.js)

- [ ] `readPanes` reads `panes.cli`.
- [ ] Build the CLI form under `panes.cli`, seed the placeholder from
      `cli.prompt`, wire submit to `verifyCli`.
- [ ] `verifyCli`: cliToPython → worker run tagged to the CLI output → on a
      clean run, `runChecks(line, cli.checks, cliOutput)`. Malformed argv fails
      immediately; a traceback fails with the exception.
- [ ] Route worker results from `__cli__.py` to `this.cliOutput` via a
      `pendingRunPane`; keep code Run landing in `this.output`.
- [ ] Generalize `runChecks` to `(text, checks, output)` so it serves both the
      infra editor and the CLI line.
- [ ] `resync` resets the CLI output/input with the others.

### Styling (styles.css)

- [ ] `.lab-cli-output` matching the other output panes; three-pane stacking.

### Content

- [ ] Add a `cli` block to "Retrieve, then Converse": prompt seeds the
      `retrieve` command; checks assert `bedrock-agent-runtime retrieve` and
      `--knowledge-base-id`, forbid the wrong plane. One step, three tools.

### Tests

- [ ] `scenario.rs`: CLI block parses; `CliPaneWithoutCall` rejects the two bad
      cases; integrity test ignores `cli.checks`.
- [ ] `render.rs`: CLI block → `panes.cli` true and a rendered CLI region;
      without → not.
- [ ] `cargo test`, clippy `-D warnings`, `cargo fmt --check`,
      `node --check assets/gen-ai/workbench.js`.

## Amendment 2 — supersedes the dual-editor and CLI-pane tasks above

Shipped. The dual-editor and CLI-pane task lists above are withdrawn; this is
what was done.

### Model (scenario.rs) — done

- [x] `Workbench` is `files`/`open`/`responses`/`reasons`/`checks`. Removed
      `open_infra`, `cli`, `CliPane`, and the infra/CLI guards + error
      variants.
- [x] `is_runnable()` kept; `has_checks()` kept. Guards reduce to
      `ConfigStepHasNoChecks` and `RunnableStepHasNoPython`.
- [x] Integrity test matches `checks` against the typescript snippets.

### Payload + template (render.rs) — done

- [x] `workbench_json` emits `runnable` + `checks`, no `panes`.
- [x] One editor/output/terminal; removed the infra editor containers.

### Browser (workbench.js) — done

- [x] One editor + buffer Map over every file; file tree switches all of them.
- [x] `updateAction`/`action` swap Run↔Check by extension; reference files
      disable the button.
- [x] Terminal under `this.runnable`: `$` prompt, neutral placeholder,
      `runCommand` → `cliToPython`, honest message on cdk/malformed, result to
      the terminal pane via `pendingRunPane`.
- [x] `resync` reseeds the one editor and terminal.

### Content — done

- [x] Former dual step keeps `.py` + `.ts` in `files`, `open` on the `.py`,
      plus `responses` and `checks`. No `openInfra`/`cli`.

### Tests — done

- [x] `scenario.rs`: both-languages step parses/runnable; pure config
      parses/not; teaches-nothing fails.
- [x] `render.rs`: payload carries both files + checks; terminal only on
      runnable; no infra containers in HTML.
- [x] `cargo test` (50 pass), clippy `-D warnings` clean, `cargo fmt --check`
      clean, `node --check` clean.

### Browser verify (manual, still to do)

- [ ] One editor; opening `.py` shows Run, `.ts` shows Check, `NOTES.md`
      disables the button.
- [ ] Terminal: `aws bedrock-agent-runtime retrieve …` runs and prints the
      intercepted result; a malformed `aws` call prints a real botocore error;
      `cdk deploy` prints the honest can't-run-here message.
- [ ] Editor Run output and terminal output do not clobber each other.
- [ ] A Next between steps reseeds the editor and terminal; a pure config step
      shows no terminal.
