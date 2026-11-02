# Phase 2 — Browser verification walkthrough

The checklist in `tasks.md` is the index; this is the how. Every expected
result below was confirmed against the real vendored Pyodide and botocore (via
the reproduction harness and the committed `smoke-test.mjs`), not assumed. An
earlier draft of this file guessed, and several guesses were wrong — the
`maxTokens` exercise did not fail, the `global.` exercise did not fail, and the
wrong-client exercise raised a different error than claimed. What follows is
what actually happens.

The things only a browser can confirm are the UI behaviors: the editor, the
dropdown, the swap-vs-navigate, the worker termination. The Python/botocore
behaviors are already covered by the smoke test in CI; they are repeated here
so a manual run is self-contained.

## Setup

1. From the repo root, after vendoring (`node assets/gen-ai/vendor-pyodide.mjs`
   if not already done), start the lab:

   ```
   cargo run -p gen-ai-handler
   ```

   It prints `listening on http://127.0.0.1:3001/gen-ai`. Plain HTTP, no AWS,
   no credentials.

2. Open `http://127.0.0.1:3001/gen-ai`. The landing lists one project,
   "EU policy assistant".

3. Open devtools, keep the Network tab visible from the start.

4. Click into the project. You land on step 1, "The system" — the architecture
   overview, one column, no workbench. Click Next to reach step 2,
   "Retrieve, then Converse", the only call step. The two-column layout
   appears: walkthrough left, workbench right.

5. The output pane shows "Loading the runtime…" then "Ready. Edit a file and
   Run it." First load fetches Pyodide, ssl/openssl, and 9 wheels from the lab
   origin. If it never reaches "Ready", stop — see Troubleshooting.

The editor opens on `answer.py` (a scaffold comment). `NOTES.md` holds the
break-it exercises. The three snippets to assemble are in the "Write the call"
pane on the left.

## The happy path

### 1. The assembled call runs and prints the response

Type the three snippets from the "Write the call" pane into `answer.py` in
order: the two clients + `QUESTION`, the `retrieve` + the loop, the `converse`
+ the two prints. Click Run.

**Pass (exact):** three lines of output —
```
s3://policy-corpus-eu/hr/parental-leave-de-v7.pdf
end_turn
German employees may take up to 156 weeks of Elternzeit per child, up to the child's eighth birthday, with up to 24 months deferrable. Source: hr/parental-leave-de-v7.pdf.
```
No traceback. This is the real botocore validating and signing the call, with
the intercept returning the authored response in place of the network hop.

## The break-it exercises (from NOTES.md)

Each was confirmed against real botocore. The point of the lab is that the
genuine library decides, so these are the library's real behaviors.

### 2. Dropping `modelId` raises ParamValidationError

In the Converse call, delete the `modelId=...` line. Run.

**Pass (exact):** a traceback ending
```
botocore.exceptions.ParamValidationError: Parameter validation failed:
Missing required parameter in input: "modelId"
```
with the lesson's reason beside it. `modelId` is the one required Converse
parameter, so botocore rejects the call during validation, before the intercept
is reached. Restore the line.

### 3. Dropping `inferenceConfig` does NOT fail — and that is the lesson

Restore `modelId`, then delete the `inferenceConfig={"maxTokens": 512}` line
(and the trailing comma above it). Run.

**Pass:** it succeeds, printing the same answer as check 1. No error.
`inferenceConfig` is optional; an unset `maxTokens` is accepted, and an
unbounded response is the cost mistake no exception warns you about. The
NOTES.md call this out explicitly — the dangerous call is the one that
succeeds. Restore the line.

### 4. Converse on the agent client raises AttributeError

Change the Converse call to use the `agent` client: `agent.converse(...)`. Run.

**Pass (exact):** a traceback ending
```
AttributeError: 'AgentsforBedrockRuntime' object has no attribute 'converse'
```
with the lesson's reason beside it. The agent client has no `converse` method,
so this fails in Python before any request is built — not a service error.
Restore `runtime.converse`.

### What the sandbox cannot show

A `global.` model-id prefix (or any residency/policy violation) is validated
server-side at AWS. The transport here is intercepted and never reaches a
server, so such a call **succeeds in the lab** even though real AWS would
reject it with a `ValidationException`. This is a deliberate boundary, stated
in NOTES.md: the sandbox teaches request shape and client-side validation, not
service-side policy. Do not expect the `global.` change to error.

### 5. A traceback's line number matches the gutter

Using check 2 (drop `modelId`), read the line the traceback attributes to
`<your code>`.

**Pass:** that line number points at the same line in the editor, counting by
the left gutter. The run harness compiles the file with the filename
`<your code>`, so the traceback uses the editor's own line numbers.

### 6. A runaway loop can be stopped

Add `while True: pass` as the first code line. Run; the Stop button enables.
Click Stop.

**Pass:** the output shows "Stopped.", the page stays responsive, and no
reload was needed. The worker was terminated and rebuilt; a subsequent Run on a
restored file works, and the filesystem reseeded. Reload to get a clean
`answer.py` before the next checks.

## The typeahead

### 7. `boto3.` completes against the real library

Type `import boto3`, then on a new line `boto3.` and stop at the dot.

**Pass:** a dropdown lists real boto3 attributes. Confirmed to include
`client`, `compat`, `crt`, `data`, `DEFAULT_SESSION` among others — whatever
the pinned botocore exposes, since jedi reads the live module. Build a client
(`c = boto3.client("bedrock-runtime")`), Run once so it exists, then `c.` lists
its operation methods (`converse`, `invoke_model`, …).

### 8. The dropdown is keyboard-only operable

With the dropdown open: Down/Up move the highlight, Enter or Tab accept and
insert, Escape dismisses.

**Pass:** all four work, the highlighted row is visibly distinct, focus stays
in the editor, and after accepting the caret sits right after the inserted
text. No mouse needed.

### 9. A completion during a run does not freeze the editor

Run a slow cell (`import time` then `time.sleep(5)`). While it runs, type a dot
after an identifier to trigger a completion.

**Pass:** the editor stays responsive; either no dropdown appears or it appears
late without inserting on its own. Completion shares the one worker with Run,
so a request during a run is expected to be delayed; what must not happen is a
freeze or a stale insertion after the caret moved.

## Navigation and the CSP boundary

### 10. Previous from the call step to the overview is a full navigation

From step 2 (call step), click Previous to step 1 (overview).

**Pass:** the Network tab shows a full **document** load of
`/gen-ai/policy-assistant` (an HTML navigation), not a POST to `/lesson`. The
overview is under the style-only policy and the call step under the widened
one; a fragment swap cannot change the document's CSP, so the script navigates.
If you see a `/lesson` fetch here, the boundary check is broken.

### 11. Next/Previous between two call steps is a swap

> The scenario has one call step today, so this is only fully exercisable once a
> second call step exists. Skip, or add a throwaway second call step locally.

Edit `answer.py`, Next to a second call step, then Previous back.

**Pass:** the Network tab shows a POST to `/gen-ai/policy-assistant/lesson`
(the fragment endpoint), not a document load, and your edit to `answer.py` is
still there — the workbench stayed mounted across the swap.

### 12. A reload resets the filesystem to the scaffold

Edit `answer.py`, then reload (F5).

**Pass:** `answer.py` is back to its scaffold comment; your assembled call is
gone. The filesystem is in-memory and seeded per step by design.

### 13. Everything loads from the lab origin, nothing hits AWS

Clear the Network tab, reload the call step, let the runtime boot, Run the
call.

**Pass:**
- `worker.js`, `pyodide.js`, the wasm, `pyodide-lock.json`, ssl/openssl, and
  the 9 wheels all come from `127.0.0.1:3001/gen-ai/assets/...` (first visit)
  or cache. No request to `cdn.jsdelivr.net`, PyPI, or any `amazonaws.com`
  host.
- Running the call produces no network entries for `retrieve`/`converse` — the
  transport is intercepted in-process.

## Troubleshooting

- **Never reaches "Ready".** Check the Network tab for 404s under
  `/gen-ai/assets/pyodide/`. If the runtime is not vendored there, the worker
  cannot load it — run `vendor-pyodide.mjs`. If the files are present, check
  the console for a `loadPackage`/import error, which points to a Pyodide
  version mismatch between the vendored files and `PYODIDE_BASE`.
- **A different error than documented.** The expected results above are pinned
  to the vendored botocore version. If you bump it, re-confirm against the
  smoke test (`node assets/gen-ai/smoke-test.mjs`), which exercises the same
  behaviors, and update this file if botocore's validation changed.

## Recording the result

Tick each box in `tasks.md` as it passes. If one fails, note what you saw next
to it. The smoke test in CI already covers checks 1–4 and 7 headlessly, so a
failure there is caught before this manual pass; this walkthrough is for the UI
behaviors (5, 6, 8–13) the smoke test cannot reach.
