// The GenAI lab workbench.
//
// Loads real CPython via Pyodide, mounts the step's seed files, installs the
// genuine boto3 with its transport intercepted, and runs the visitor's code
// in a web worker so a runaway loop can be stopped without the page.
//
// Nothing here is a simulation of Python. The only thing faked is the network
// hop: botocore builds the client, validates the call, and serializes the
// request for real, then the intercept returns the step's authored response
// instead of reaching a socket that does not exist.
//
// This file is browser-verified, not test-covered: the Rust side cannot run
// Pyodide. The checklist in phase2/tasks.md is its test.

'use strict';

// Self-hosted on this origin, not a CDN. The runtime, its wasm, and the wheels
// are vendored into the lab's asset bucket and served under this path behind
// CloudFront, so the page stays under `script-src 'self'`/`connect-src 'self'`
// with no third-party vendor. loadPyodide's indexURL and micropip both read
// from here. The trailing slash matters: Pyodide concatenates filenames onto
// it.
const PYODIDE_BASE = '/gen-ai/assets/pyodide/';

// The wheels micropip installs, as same-origin URLs under PYODIDE_BASE. An
// explicit `.whl` URL is how micropip installs without reaching PyPI;
// dependencies still resolve, so the full transitive set for boto3, botocore,
// and jedi must be vendored beside these and present in the index. The exact
// filenames are pinned by the vendor step that runs micropip once against the
// target Pyodide release and records what it fetched; see
// phase2/self-hosting.md. Until that step is run and this list is filled,
// boot() falls back to installing by name against PYODIDE_BASE as an index.
const WHEELS = [
  'boto3-1.43.123-py3-none-any.whl',
  'botocore-1.43.123-py3-none-any.whl',
  'jedi-0.19.1-py2.py3-none-any.whl',
  'jmespath-1.1.0-py3-none-any.whl',
  'parso-0.8.4-py2.py3-none-any.whl',
  'python_dateutil-2.9.0.post0-py2.py3-none-any.whl',
  's3transfer-0.19.3-py3-none-any.whl',
  'six-1.16.0-py2.py3-none-any.whl',
  'urllib3-2.2.1-py3-none-any.whl',
];

// --- reading the step --------------------------------------------------------

/** The inert JSON the Rust side rendered. */
function readWorkbench() {
  const node = document.getElementById('lab-workbench-data');
  if (!node) return null;
  try {
    return JSON.parse(node.textContent);
  } catch {
    return null;
  }
}

// --- mounting ----------------------------------------------------------------

class Workbench {
  constructor(root, data) {
    this.root = root;
    // The data block is { workbench, workerUrl }. Keep the workbench under
    // `data` so the rest of this class reads data.files/open/responses, and
    // hold the worker URL separately.
    this.data = data.workbench;
    this.workerUrl = data.workerUrl;
    // A runnable step has an intercept to submit against, so it gets the
    // terminal; a pure configuration step does not. The editor always renders.
    this.runnable = data.runnable !== false;
    this.checks = data.checks || [];
    this.worker = null;
    this.ready = false;
    this.openPath = this.data.open;
    // Typeahead state. seq rises per request so a late reply is dropped;
    // active is the open dropdown or null.
    this.completionSeq = 0;
    this.completion = null;
    this.completionTimer = null;
    this.render();
  }

  /** Re-point the workbench at a new step after the nav script swaps the
   *  lesson. The swap replaces only the lesson region, so this is what keeps
   *  the workbench beside it from grading a config step against the previous
   *  step's answer key or showing a stale seed file.
   *
   *  A booted runtime is torn down: seed files, the intercept's responses, and
   *  the runnable/config distinction are all per-step, so the worker from the
   *  old step cannot be reused for the new one. render() reboots it when the
   *  new step is runnable. The reboot cost falls only between two runnable
   *  steps; config-to-anything never had a worker to lose. */
  resync(data) {
    if (this.worker) {
      try {
        this.worker.terminate();
      } catch {
        // Already gone; nothing to do.
      }
      this.worker = null;
    }
    this.ready = false;
    if (this.menu) this.closeCompletions();
    this.data = data.workbench;
    this.runnable = data.runnable !== false;
    this.checks = data.checks || [];
    this.openPath = this.data.open;
    this.render();
  }

  /** Whether the file at `path` runs Python against the intercept (`.py`) or
   *  is checked against the answer key (`.ts`). The action button follows
   *  this, so a single editor serves both: open a .py file and the button is
   *  Run, open a .ts file and it is Check. A file that is neither (NOTES.md)
   *  is readable but has no action, which `isRunnablePath`/`isCheckPath`
   *  below both report false for. */
  isRunnablePath(path) {
    return path.endsWith('.py');
  }

  isCheckPath(path) {
    return path.endsWith('.ts');
  }

  render() {
    const tree = this.root.querySelector('.lab-files');
    const editor = this.root.querySelector('.lab-editor');
    const output = this.root.querySelector('.lab-output');
    const cli = this.root.querySelector('.lab-cli');
    const cliOutput = this.root.querySelector('.lab-cli-output');

    // Drop the no-script fallback once we are live.
    const fallback = this.root.querySelector('noscript');
    if (fallback) fallback.remove();

    // One editor, one buffer Map, every seeded file in it. The file tree
    // switches between all of them — .py, .ts, and NOTES.md alike — and the
    // action button follows whichever is open. There is no second editor and
    // no file held out of the tree, so there is no pair of buffers to fight
    // over one file.
    tree.innerHTML = '';
    const list = document.createElement('ul');
    list.className = 'lab-file-list';
    for (const f of this.data.files) {
      const li = document.createElement('li');
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.textContent = f.path;
      btn.setAttribute('aria-pressed', String(f.path === this.openPath));
      btn.addEventListener('click', () => this.open(f.path));
      li.appendChild(btn);
      list.appendChild(li);
    }
    tree.appendChild(list);

    // One buffer per file. Switching tabs saves the current buffer into this
    // Map and loads the next, so an edit to the .ts file survives a detour
    // through the .py file and back.
    this.files = new Map(this.data.files.map((f) => [f.path, f.body]));

    // Reset editor state a previous step's render left behind.
    this.textarea = null;
    this.gutter = null;
    this.menu = null;
    this.output = output;
    this.cliOutput = null;
    this.cliInput = null;
    // Which pane a pending worker run writes its result to. The editor Run and
    // the terminal run share one worker and one filesystem; the path they
    // write disambiguates, but the result message does not carry it, so the
    // pane is tracked here between the postMessage and the reply.
    this.pendingRunPane = 'editor';
    editor.innerHTML = '';
    output.textContent = '';
    cli.innerHTML = '';
    if (cliOutput) cliOutput.textContent = '';

    // The single editor: a textarea with a line-number gutter (a traceback
    // names a line, so the gutter is function) and the completion dropdown.
    const built = this.buildCodeEditor(editor);
    this.textarea = built.textarea;
    this.gutter = built.gutter;
    this.menu = built.menu;

    // The action button follows the open file's extension, so its label is
    // set in `open()` rather than here. Run executes the buffer against the
    // intercept; Check grades it against the answer key. One button, two jobs.
    const actionBtn = document.createElement('button');
    actionBtn.type = 'button';
    actionBtn.className = 'lab-run';
    actionBtn.addEventListener('click', () => this.action());
    this.actionBtn = actionBtn;

    const stopBtn = document.createElement('button');
    stopBtn.type = 'button';
    stopBtn.className = 'lab-stop';
    stopBtn.textContent = 'Stop';
    stopBtn.disabled = true;
    stopBtn.addEventListener('click', () => this.stop());
    this.stopBtn = stopBtn;

    editor.appendChild(actionBtn);
    editor.appendChild(stopBtn);

    // The terminal: always present on a runnable step, where the intercept it
    // submits to exists. One `aws` line, no pre-fill and no authored answer
    // key — the visitor types a full command and botocore's acceptance or
    // rejection is the whole verification. A pure configuration step has no
    // intercept, so no terminal.
    if (this.runnable) {
      this.cliOutput = cliOutput;
      const form = document.createElement('form');
      form.className = 'lab-cli__form';
      // A shell `$`, not an `aws` label: this reads as a terminal, and the
      // placeholder is what conveys that it runs aws commands.
      const prompt = document.createElement('span');
      prompt.className = 'lab-cli__prompt';
      prompt.textContent = '$';
      prompt.setAttribute('aria-hidden', 'true');
      const input = document.createElement('input');
      input.type = 'text';
      input.className = 'lab-cli__input';
      input.setAttribute('aria-label', 'Terminal command');
      // A neutral, static placeholder. No value seeded from content, so the
      // visitor composes the command themselves.
      input.placeholder = 'aws <service> <operation> --flag value';
      form.appendChild(prompt);
      form.appendChild(input);
      form.addEventListener('submit', (ev) => {
        ev.preventDefault();
        this.runCommand(input.value);
      });
      cli.appendChild(form);
      this.cliInput = input;
      if (this.cliOutput) {
        this.cliOutput.textContent =
          'Terminal: type an aws command and submit. botocore validates it ' +
          'against the same sandbox the editor runs against.';
      }
    }

    // Open the starting file; this sets the editor buffer and the action
    // button label.
    this.open(this.openPath);

    // The runtime only exists for a runnable step. A pure configuration step
    // grades its .ts file with the in-browser substring checks and never boots
    // Pyodide, so it gets a Check hint rather than a loading message.
    if (this.runnable) {
      this.output.textContent = 'Loading the runtime…';
      this.boot();
    } else {
      this.output.textContent =
        'Configuration step: assemble the artifact from the snippets, then ' +
        'Check to verify it has what this decision requires.';
    }
  }

  /** Build the editor (gutter + textarea + completion menu) inside a
   *  container. Returns the created elements; the caller stores them and wires
   *  the action button and Stop. */
  buildCodeEditor(container) {
    const wrap = document.createElement('div');
    wrap.className = 'lab-editor__wrap';
    const gutter = document.createElement('div');
    gutter.className = 'lab-editor__gutter';
    gutter.setAttribute('aria-hidden', 'true');
    const textarea = document.createElement('textarea');
    textarea.className = 'lab-editor__area';
    textarea.spellcheck = false;
    textarea.setAttribute('aria-label', 'Code editor');
    textarea.setAttribute('role', 'combobox');
    textarea.setAttribute('aria-autocomplete', 'list');
    textarea.setAttribute('aria-expanded', 'false');
    textarea.addEventListener('input', () => {
      this.syncGutter();
      this.requestCompletions();
    });
    textarea.addEventListener('scroll', () => {
      gutter.scrollTop = textarea.scrollTop;
      this.closeCompletions();
    });
    textarea.addEventListener('blur', () => {
      setTimeout(() => this.closeCompletions(), 120);
    });
    textarea.addEventListener('keydown', (ev) => this.onEditorKey(ev));
    wrap.appendChild(gutter);
    wrap.appendChild(textarea);

    const menu = document.createElement('ul');
    menu.className = 'lab-complete';
    menu.id = 'lab-complete';
    menu.setAttribute('role', 'listbox');
    menu.setAttribute('aria-label', 'Completions');
    menu.hidden = true;
    textarea.setAttribute('aria-controls', 'lab-complete');
    wrap.appendChild(menu);

    container.appendChild(wrap);
    return { wrap, textarea, gutter, menu };
  }

  open(path) {
    if (!this.textarea) return;

    // Save the current buffer only when switching away from a *different*
    // file. Guarding on the path change is what stops the first mount, where
    // openPath already equals path, from writing the empty textarea over the
    // seed before it has been loaded.
    if (this.openPath && this.openPath !== path) {
      this.files.set(this.openPath, this.textarea.value);
    }
    this.openPath = path;
    this.textarea.value = this.files.get(path) ?? '';
    this.syncGutter();
    for (const btn of this.root.querySelectorAll('.lab-file-list button')) {
      btn.setAttribute('aria-pressed', String(btn.textContent === path));
    }
    this.updateAction(path);
  }

  /** Set the action button's label and enabled state from the open file's
   *  extension, and hint the output pane at what the button will do. A .py
   *  file runs against the intercept; a .ts file is checked against the answer
   *  key. A file that is neither (NOTES.md) has no action, so the button is
   *  disabled — the file is readable reference, not something to run. */
  updateAction(path) {
    if (!this.actionBtn) return;
    if (this.isRunnablePath(path)) {
      this.actionBtn.textContent = 'Run';
      this.actionBtn.disabled = !this.runnable;
      if (!this.runnable) {
        // A .py file on a non-runnable step cannot run — there is no
        // intercept. In practice a config step seeds no .py, so this is a
        // guard rather than a path a visitor reaches.
        this.actionBtn.title = 'This step runs no live call.';
      } else {
        this.actionBtn.removeAttribute('title');
      }
    } else if (this.isCheckPath(path)) {
      this.actionBtn.textContent = 'Check';
      this.actionBtn.disabled = false;
      this.actionBtn.removeAttribute('title');
    } else {
      // Reference file (notes, data). Nothing to Run or Check.
      this.actionBtn.textContent = this.runnable ? 'Run' : 'Check';
      this.actionBtn.disabled = true;
      this.actionBtn.title = 'This file is reference only.';
    }
  }

  /** The action button. Dispatches on the open file's extension: Run a .py
   *  buffer against the intercept, Check a .ts buffer against the answer key.
   *  A file that is neither has a disabled button and never reaches here. */
  action() {
    const path = this.openPath;
    if (this.isRunnablePath(path)) {
      this.run();
    } else if (this.isCheckPath(path)) {
      // Persist the buffer first so the check grades exactly what is shown.
      this.files.set(path, this.textarea.value);
      this.runChecks(this.textarea.value, this.checks, this.output);
    }
  }

  syncGutter() {
    if (!this.textarea || !this.gutter) return;
    const lines = this.textarea.value.split('\n').length;
    let out = '';
    for (let i = 1; i <= lines; i++) out += i + '\n';
    this.gutter.textContent = out;
  }

  // --- typeahead -------------------------------------------------------------

  /** Keydown on the editor. The dropdown, when open, owns the navigation
   *  keys; otherwise Tab inserts spaces as before. */
  onEditorKey(ev) {
    if (this.completion) {
      if (ev.key === 'ArrowDown') {
        ev.preventDefault();
        this.moveCompletion(1);
        return;
      }
      if (ev.key === 'ArrowUp') {
        ev.preventDefault();
        this.moveCompletion(-1);
        return;
      }
      if (ev.key === 'Enter' || ev.key === 'Tab') {
        // Tab and Enter accept the highlighted item when the menu is open.
        ev.preventDefault();
        this.acceptCompletion();
        return;
      }
      if (ev.key === 'Escape') {
        ev.preventDefault();
        this.closeCompletions();
        return;
      }
    }

    // Tab inserts spaces, because Python. Only when no menu claimed it.
    if (ev.key === 'Tab') {
      ev.preventDefault();
      const s = this.textarea.selectionStart;
      const e = this.textarea.selectionEnd;
      this.textarea.value =
        this.textarea.value.slice(0, s) + '    ' + this.textarea.value.slice(e);
      this.textarea.selectionStart = this.textarea.selectionEnd = s + 4;
      this.syncGutter();
    }
  }

  /** Ask the worker for completions at the caret, debounced. Only fires with
   *  the runtime up and the caret sitting on a word or just after a dot, so
   *  it stays quiet in whitespace and mid-sentence. */
  requestCompletions() {
    if (this.completionTimer) clearTimeout(this.completionTimer);
    if (!this.ready) {
      this.closeCompletions();
      return;
    }

    const value = this.textarea.value;
    const caret = this.textarea.selectionStart;
    const prefix = value.slice(0, caret);
    // A trigger is an identifier character or a dot immediately behind the
    // caret. Anything else (space, newline, an open paren) closes the menu.
    if (!/[\w.]$/.test(prefix)) {
      this.closeCompletions();
      return;
    }

    this.completionTimer = setTimeout(() => {
      const { line, column } = this.caretLineColumn();
      const seq = ++this.completionSeq;
      this.pendingCompletion = { seq, caret, value };
      this.worker.postMessage({
        kind: 'complete',
        seq,
        source: value,
        line,
        column,
      });
    }, 120);
  }

  /** Caret as a zero-based line and column, which is what jedi's column and
   *  our +1 on the line expect. */
  caretLineColumn() {
    const upto = this.textarea.value.slice(0, this.textarea.selectionStart);
    const lines = upto.split('\n');
    return { line: lines.length - 1, column: lines[lines.length - 1].length };
  }

  onCompletions(m) {
    // Drop a reply the buffer has moved past, or one for a request that is no
    // longer the latest.
    if (!this.pendingCompletion || m.seq !== this.pendingCompletion.seq) return;
    if (this.textarea.selectionStart !== this.pendingCompletion.caret) return;
    if (this.textarea.value !== this.pendingCompletion.value) return;

    const items = (m.items || []).filter((it) => it && it.name);
    if (items.length === 0) {
      this.closeCompletions();
      return;
    }
    this.openCompletions(items);
  }

  openCompletions(items) {
    this.menu.innerHTML = '';
    items.forEach((item, i) => {
      const li = document.createElement('li');
      li.className = 'lab-complete__item';
      li.id = `lab-complete-${i}`;
      li.setAttribute('role', 'option');
      li.setAttribute('aria-selected', String(i === 0));

      const name = document.createElement('span');
      name.className = 'lab-complete__name';
      name.textContent = item.name;
      const kind = document.createElement('span');
      kind.className = 'lab-complete__kind';
      kind.textContent = item.type || '';

      li.appendChild(name);
      li.appendChild(kind);
      // mousedown, not click: it fires before the textarea's blur, so the
      // item is chosen before the deferred close can run.
      li.addEventListener('mousedown', (ev) => {
        ev.preventDefault();
        this.index = i;
        this.acceptCompletion();
      });
      this.menu.appendChild(li);
    });

    this.completion = { items };
    this.index = 0;
    this.menu.hidden = false;
    this.textarea.setAttribute('aria-expanded', 'true');
    this.positionMenu();
    this.highlightCompletion();
  }

  closeCompletions() {
    if (this.completionTimer) clearTimeout(this.completionTimer);
    this.completion = null;
    this.pendingCompletion = null;
    this.menu.hidden = true;
    this.menu.innerHTML = '';
    this.textarea.setAttribute('aria-expanded', 'false');
    this.textarea.removeAttribute('aria-activedescendant');
  }

  moveCompletion(delta) {
    const n = this.completion.items.length;
    this.index = (this.index + delta + n) % n;
    this.highlightCompletion();
  }

  highlightCompletion() {
    const options = this.menu.querySelectorAll('.lab-complete__item');
    options.forEach((li, i) => {
      const on = i === this.index;
      li.setAttribute('aria-selected', String(on));
      li.classList.toggle('is-active', on);
      if (on) {
        this.textarea.setAttribute('aria-activedescendant', li.id);
        li.scrollIntoView({ block: 'nearest' });
      }
    });
  }

  /** Insert the highlighted item's remaining suffix at the caret. jedi's
   *  `complete` is exactly the text still to type, so there is no prefix to
   *  re-derive or delete. */
  acceptCompletion() {
    if (!this.completion) return;
    const item = this.completion.items[this.index];
    const suffix = item.complete ?? '';
    const caret = this.textarea.selectionStart;
    const value = this.textarea.value;
    this.textarea.value = value.slice(0, caret) + suffix + value.slice(caret);
    const next = caret + suffix.length;
    this.textarea.selectionStart = this.textarea.selectionEnd = next;
    this.closeCompletions();
    this.syncGutter();
    this.textarea.focus();
  }

  /** Put the menu under the caret's line. A textarea exposes no caret
   *  rectangle, so the row comes from the line count and the column from the
   *  character offset, both in the editor's own monospace metrics. Good
   *  enough for a dropdown that only needs to sit near the word. */
  positionMenu() {
    const { line, column } = this.caretLineColumn();
    const style = getComputedStyle(this.textarea);
    const lineHeight = parseFloat(style.lineHeight) || 18;
    const charWidth = this.charWidth();
    const padTop = parseFloat(style.paddingTop) || 0;
    const padLeft = parseFloat(style.paddingLeft) || 0;

    const top =
      padTop + (line + 1) * lineHeight - this.textarea.scrollTop;
    const left = padLeft + column * charWidth - this.textarea.scrollLeft;

    this.menu.style.setProperty('--lab-complete-top', `${top}px`);
    this.menu.style.setProperty('--lab-complete-left', `${left}px`);
  }

  /** Width of one monospace character, measured once and cached. */
  charWidth() {
    if (this._charWidth) return this._charWidth;
    const probe = document.createElement('span');
    const style = getComputedStyle(this.textarea);
    probe.style.font = style.font;
    probe.style.position = 'absolute';
    probe.style.visibility = 'hidden';
    probe.style.whiteSpace = 'pre';
    probe.textContent = '0000000000';
    document.body.appendChild(probe);
    this._charWidth = probe.getBoundingClientRect().width / 10;
    probe.remove();
    return this._charWidth;
  }

  boot() {
    try {
      // From the served, content-hashed URL rather than a Blob. The worker is
      // same-origin, so `worker-src 'self'` covers it with no `blob:`, and
      // importScripts resolves its paths against this origin normally.
      this.worker = new Worker(this.workerUrl);
    } catch (err) {
      // The editor is already painted, so report into the output pane rather
      // than letting a construction failure bubble up and blank the mount.
      this.output.textContent =
        'The runtime worker could not start (check worker-src in the page CSP):\n' +
        String(err);
      return;
    }
    // A worker that fails to load its scripts (e.g. a 404 on the vendored
    // runtime) fires onerror rather than posting a message. Surface it.
    this.worker.onerror = (e) => {
      this.output.textContent =
        'The runtime failed to load. If the Pyodide files are not yet ' +
        'vendored under /gen-ai/assets/pyodide/, run vendor-pyodide.mjs.\n' +
        String(e.message || e);
    };
    this.worker.onmessage = (e) => this.onMessage(e.data);
    this.worker.postMessage({
      kind: 'boot',
      // Absolute, resolved against the page origin on the main thread, so the
      // worker's importScripts and Pyodide's fetches have a fully-qualified
      // base regardless of how the worker itself was loaded.
      base: new URL(PYODIDE_BASE, location.origin).href,
      files: this.data.files,
      // The authored responses, as plain data. The worker substitutes them
      // into its intercept; no intercept source crosses the boundary.
      responses: this.data.responses,
      packages: ['boto3', 'botocore', 'jedi'],
      wheels: WHEELS,
    });
  }

  onMessage(m) {
    if (m.kind === 'ready') {
      this.ready = true;
      this.output.textContent = 'Ready. Edit a file and Run it.';
    } else if (m.kind === 'output') {
      if (this.pendingRunPane === 'terminal') {
        this.finishCommand(m);
      } else {
        this.showOutput(m);
      }
      this.running(false);
    } else if (m.kind === 'traceback') {
      this.pendingTraceback = m.text;
    } else if (m.kind === 'fatal') {
      this.output.textContent = 'The runtime failed to start: ' + m.text;
      this.running(false);
    } else if (m.kind === 'completions') {
      this.onCompletions(m);
    }
  }

  showOutput(m) {
    const parts = [];
    if (m.stdout) parts.push(m.stdout);
    if (this.pendingTraceback) {
      parts.push(this.pendingTraceback);
      this.pendingTraceback = null;
    } else if (m.stderr) {
      parts.push(m.stderr);
    }
    this.output.textContent = parts.join('\n') || '(no output)';
  }

  /** The reply to a terminal submit. There is no authored answer key here:
   *  botocore's own acceptance or rejection is the whole verification. A
   *  traceback means the library rejected the command — a wrong operation, a
   *  missing required parameter, the wrong client — which is a real error the
   *  visitor should see and fix. A clean reply means botocore accepted and
   *  serialized the call and the intercept returned the step's response, which
   *  is printed as-is. Either way the output is the result, with no pass/fail
   *  layered on top. */
  finishCommand(m) {
    const out = this.cliOutput;
    if (!out) return;
    if (this.pendingTraceback) {
      const tb = this.pendingTraceback;
      this.pendingTraceback = null;
      out.textContent =
        '\u2717 The command was rejected before it ran. botocore validated it ' +
        'against the real client and raised:\n\n' +
        tb +
        '\n\nFix the command and submit again.';
      return;
    }
    this.showCommandOutput(m);
  }

  /** The terminal's own output. Like `showOutput`, but writes the terminal
   *  pane rather than the editor's, so a command never clobbers a Run or
   *  Check result above it. */
  showCommandOutput(m) {
    const out = this.cliOutput;
    if (!out) return;
    const parts = [];
    if (m.stdout) parts.push(m.stdout);
    if (m.stderr) parts.push(m.stderr);
    out.textContent = parts.join('\n') || '(no output)';
  }

  run() {
    if (!this.ready) return;
    // An editor Run routes its result to the editor output, even if the
    // previous action was a terminal command that left the pane pointer on
    // 'terminal'.
    this.pendingRunPane = 'editor';
    // Persist the buffer to the worker's filesystem, then run it.
    this.files.set(this.openPath, this.textarea.value);
    this.worker.postMessage({ kind: 'write', path: this.openPath, body: this.textarea.value });
    this.worker.postMessage({ kind: 'run', path: this.openPath });
    this.running(true);
    this.output.textContent = 'Running…';
  }

  /** Check a .ts buffer against the step's structural checks. No runtime: each
   *  check is a substring that must be present or absent in what the visitor
   *  typed, with the decision it verifies. The absence checks are the ones
   *  that catch a left-in mistake — a wildcard IAM, a fine-tune job — that a
   *  'does it contain' test alone would miss. */
  runChecks(text, checks, out, opts) {
    if (out == null || typeof text !== 'string') return;
    checks = checks || [];
    const lines = [];
    let allPassed = true;
    for (const check of checks) {
      const found = text.includes(check.pattern);
      const passed = check.present ? found : !found;
      if (!passed) allPassed = false;
      lines.push((passed ? '\u2713 ' : '\u2717 ') + check.message);
    }
    const header = allPassed
      ? 'All checks passed. The artifact has what this decision requires.'
      : 'Some checks did not pass. Read the \u2717 lines and adjust the artifact.';
    const preamble = opts && opts.preamble ? opts.preamble + '\n\n' : '';
    out.textContent = preamble + header + '\n\n' + lines.join('\n');
  }

  /** Run a terminal command. An `aws` command is translated by the shim and
   *  run against the same intercept the editor uses, so botocore validates it
   *  for real; its output is the verification, with no authored answer key. A
   *  command the shim cannot translate — a `cdk` line, or anything `cliToPython`
   *  returns null for — is not runnable here, so it prints a plain message
   *  rather than failing silently. The result lands in the terminal output
   *  pane, never the editor output. */
  runCommand(line) {
    if (!this.ready) return;
    const out = this.cliOutput;
    if (!out) return;
    if (!line.trim()) {
      out.textContent = 'Type a command, then submit.';
      return;
    }
    // Only aws commands execute here. The shim strips a leading `aws` and
    // translates argv to a boto3 call; a null means it is not an aws command
    // we can run (a cdk line, or malformed argv). cdk in particular has no
    // cloud to deploy to, so say so rather than throwing or ignoring it.
    const src = cliToPython(line);
    if (!src) {
      out.textContent =
        'This terminal runs aws commands against the sandbox (botocore ' +
        'validates them). cdk commands can\u2019t execute here \u2014 there ' +
        'is no cloud to deploy to.';
      return;
    }
    this.pendingRunPane = 'terminal';
    this.worker.postMessage({ kind: 'write', path: '__cli__.py', body: src });
    this.worker.postMessage({ kind: 'run', path: '__cli__.py' });
    this.running(true);
    out.textContent = 'Running the command against the sandbox\u2026';
  }

  running(on) {
    this.stopBtn.disabled = !on;
  }

  stop() {
    // Terminate the worker to kill a runaway, then rebuild it. The filesystem
    // reseeds from the step, which matches the reload semantics.
    if (this.worker) this.worker.terminate();
    this.output.textContent = 'Stopped.';
    this.ready = false;
    this.running(false);
    this.boot();
  }
}

// --- the aws shim ------------------------------------------------------------
//
// argv to a boto3 call. Deliberately a shim, not awscli: the exam tests the
// shape of the call, not the CLI's own parser. An unrecognised flag is
// reported, never dropped, because silently ignoring an argument is how a
// visitor concludes a wrong call was right.
//
// Returns null for anything that is not an aws command the sandbox can run:
// a `cdk` line (no cloud to deploy to), a bare verb that is clearly another
// tool, or argv too short or malformed to be a service call. The terminal
// turns a null into a plain message rather than a fake success.

function cliToPython(line) {
  const tokens = line.trim().split(/\s+/);
  // A leading `aws` is the explicit form and is stripped. A leading verb that
  // names a different tool (notably `cdk`) is not runnable here, so reject it
  // rather than translating `cdk deploy` into a nonsense boto3 call.
  if (tokens[0] === 'aws') {
    tokens.shift();
  } else if (tokens[0] === 'cdk') {
    return null;
  }
  if (tokens.length < 2) return null;

  const service = tokens[0];
  const operation = tokens[1];
  const method = operation.replace(/-/g, '_');

  const kwargs = [];
  for (let i = 2; i < tokens.length; i++) {
    const t = tokens[i];
    if (!t.startsWith('--')) return null; // a value with no flag: reject.
    const key = t
      .slice(2)
      .split('-')
      .map((w, j) => (j === 0 ? w : w.charAt(0).toUpperCase() + w.slice(1)))
      .join('');
    const value = tokens[i + 1];
    if (value === undefined || value.startsWith('--')) return null;
    i++;
    kwargs.push(`${key}=${JSON.stringify(value)}`);
  }

  return `import boto3, json
_c = boto3.client(${JSON.stringify(service)}, region_name="eu-central-1")
_r = _c.${method}(${kwargs.join(', ')})
print(json.dumps(_r, indent=2, default=str))
`;
}

// --- entry -------------------------------------------------------------------

function mount() {
  const root = document.querySelector('.lab-workbench');
  if (!root) return; // overview, or a page with no workbench.
  const data = readWorkbench();
  if (!data) {
    showMountError(root, 'The workbench data block is missing or malformed.');
    return;
  }
  try {
    // Exposed so the nav script can re-sync the workbench to the new step
    // after it swaps the lesson region. The workbench is deliberately kept
    // mounted across a swap; without a handle to it, a Next between two call
    // steps would leave its seed files and config-step checks pointing at the
    // previous step.
    window.__labWorkbench = new Workbench(root, data);
  } catch (err) {
    // A synchronous failure in render or boot must never leave a silent black
    // pane. Surface it where the visitor is looking, not only in the console.
    showMountError(root, String((err && err.stack) || err));
  }
}

/** Paint a visible failure into the workbench rather than leaving it blank. */
function showMountError(root, message) {
  const output = root.querySelector('.lab-output') || root;
  output.textContent = 'The workbench failed to start:\n' + message;
  // eslint-disable-next-line no-console
  console.error('[workbench] mount failed:', message);
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', mount);
} else {
  mount();
}
