// Smoke test for the GenAI lab's Pyodide worker, run under Node.
//
// This is the test the Rust side cannot be: gen-ai-core forbids heavy deps and
// cannot run Pyodide, and workbench.js / worker.js only fail at runtime. The
// long chain of runtime bugs this project hit — ssl, credentials, loadPackage
// vs micropip, URL-keyed intercept, the response stream(), and the
// backtick-escaping breakage — are all things this harness catches in seconds,
// where before they surfaced only in a browser on the Nth attempt.
//
// It loads the vendored runtime, applies the real worker's intercept and
// completion setup, and runs the authored scenario end to end, asserting the
// intercepted response comes back. Requires the vendor step to have run
// (assets/gen-ai/pyodide present); CI runs the vendor step first.
//
//   node assets/gen-ai/smoke-test.mjs
//
// Exits non-zero on any failure so CI fails loudly.

import { loadPyodide } from './pyodide/pyodide.mjs';
import { readFile, readdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const pyodideDir = join(here, 'pyodide');
const wheelDir = join(pyodideDir, 'wheels');

function fail(msg) {
  console.error('SMOKE TEST FAILED: ' + msg);
  process.exit(1);
}

if (!existsSync(join(pyodideDir, 'pyodide.mjs'))) {
  fail('Pyodide is not vendored. Run assets/gen-ai/vendor-pyodide.mjs first.');
}

// Pull the intercept, completion setup, and WHEELS out of the real worker and
// workbench sources, so this tests the shipped code rather than a copy that
// can drift. The blocks are delimited by their const declarations.
function extractTemplate(source, name) {
  const start = source.indexOf('const ' + name + ' = `');
  if (start === -1) fail('could not find ' + name + ' in worker.js');
  const from = source.indexOf('`', start) + 1;
  const end = source.indexOf('`', from);
  return source.slice(from, end);
}

function extractWheels(source) {
  const m = source.match(/const WHEELS = \[([\s\S]*?)\]/);
  if (!m) fail('could not find WHEELS in workbench.js');
  return [...m[1].matchAll(/'([^']+\.whl)'/g)].map((x) => x[1]);
}

const workerSrc = await readFile(join(here, 'worker.js'), 'utf8');
const workbenchSrc = await readFile(join(here, 'workbench.js'), 'utf8');

const INTERCEPT = extractTemplate(workerSrc, 'INTERCEPT');
const COMPLETION_SETUP = extractTemplate(workerSrc, 'COMPLETION_SETUP');
const RUN_HARNESS = extractTemplate(workerSrc, 'RUN_HARNESS');
const WHEELS = extractWheels(workbenchSrc);

// Every authored scenario, read from content/ rather than copied here. This
// is the change that makes the harness scale: it runs each runnable step's
// own assembled snippets against that step's own intercepted responses, so a
// new scenario file is covered the moment it lands, with no edit here. The
// bugs this caught by hand (a client never built, a CDK snippet pasted as
// Python) become CI failures instead.
const contentDir = join(here, '..', '..', 'content', 'gen-ai');
const scenarioFiles = (await readdir(contentDir)).filter((f) => f.endsWith('.json'));
if (scenarioFiles.length === 0) fail('no scenarios found in content/gen-ai');

/** The python snippets of a step, concatenated in authored order the way a
 *  visitor assembles them into the open file. A non-python snippet (a shell
 *  sidecar, a CDK trigger shown for context) is not part of the runnable
 *  program, exactly as the visitor would not paste it into a .py file. */
function assemblePython(step) {
  return step.snippets
    .filter((s) => s.language === 'python')
    .map((s) => s.source)
    .join('\n\n');
}

/** Every runnable step across every scenario: { scenario, title, code,
 *  responses }. A runnable step is one with intercepted responses; a config
 *  step has none and is verified by its structural checks, not by running. */
const runnableSteps = [];
for (const file of scenarioFiles) {
  const doc = JSON.parse(await readFile(join(contentDir, file), 'utf8'));
  for (const step of doc.steps) {
    if (step.kind !== 'call') continue;
    const responses = step.workbench.responses || [];
    if (responses.length === 0) continue; // config step
    runnableSteps.push({
      scenario: doc.id,
      title: step.title,
      code: assemblePython(step),
      responses,
    });
  }
}
if (runnableSteps.length === 0) fail('no runnable steps found to verify');

console.log('Booting Pyodide from vendored runtime…');
const py = await loadPyodide({ indexURL: pyodideDir + '/' });

console.log('Loading ssl + ' + WHEELS.length + ' wheels…');
await py.loadPackage('ssl');
await py.loadPackage(WHEELS.map((w) => join(wheelDir, w)));

// Build the intercept exactly as boot() in worker.js does, so the smoke test
// exercises the real substitution rather than a copy that can drift. boot()
// defaults a missing responses array to [] (a config step omits it) and
// double-encodes, so the marker becomes a string literal of JSON. A single
// stringify yields a bare array Python reads as a list; an undefined without
// the ?? [] yields the literal token `undefined` and a NameError. Both were
// real bugs this mirrors.
function buildIntercept(responsesJson) {
  const responses = responsesJson ?? [];
  return INTERCEPT.replace('__RESPONSES_JSON__', JSON.stringify(JSON.stringify(responses)));
}

console.log('Installing completion setup (from worker.js)…');
await py.runPythonAsync(COMPLETION_SETUP);

// 1. Completion: typing boto3. resolves against the live library.
const complete = py.globals.get('_lab_complete');
const items = complete('import boto3\nboto3.', 1, 6).toJs({ dict_converter: Object.fromEntries });
complete.destroy();
const names = items.map((i) => i.name);
if (!names.includes('client')) {
  fail('completion did not resolve boto3.client; got: ' + names.join(', '));
}
console.log('  completion OK (boto3. -> ' + names.slice(0, 5).join(', ') + '…)');

// Every string leaf in a response body, so the "intercepted response
// surfaced" assertion can check whether any of them appears in what the step
// printed — the chunk text, the answer, a job status, a cited URI — without
// this harness knowing which field a given step chose to print. Uses the
// step's own content, not a value hardcoded here, so it scales to any
// scenario.
function stringLeaves(value, out = []) {
  if (typeof value === 'string') {
    if (value.trim()) out.push(value);
  } else if (Array.isArray(value)) {
    for (const v of value) stringLeaves(v, out);
  } else if (value && typeof value === 'object') {
    for (const v of Object.values(value)) stringLeaves(v, out);
  }
  return out;
}

// 2. Every runnable step, across every scenario, assembled and run through the
// intercept seeded with that step's own responses. This is the oracle: a step
// whose snippets do not assemble into a running program (a client never built,
// a non-python snippet pasted in) fails here, in CI, instead of in a browser.
console.log('Running ' + runnableSteps.length + ' runnable step(s) across ' + scenarioFiles.length + ' scenario(s)…');
for (const step of runnableSteps) {
  const label = step.scenario + ' / ' + step.title;

  if (!step.code.trim()) {
    fail(label + ': runnable step assembled no python to run');
  }

  // Seed the intercept with this step's own responses, exactly as boot() does.
  await py.runPythonAsync(buildIntercept(step.responses));

  py.FS.writeFile('__run_target__', step.code);
  const result = JSON.parse(await py.runPythonAsync(RUN_HARNESS));
  if (result.traceback) {
    fail(label + ' raised when assembled and run:\n' + result.traceback);
  }

  // The intercept must actually reach the output: at least one string from
  // the step's own responses appears in what it printed. This proves the
  // substitution wired through end to end without demanding the step print
  // every field (a step prints the answer, not the token counts). A step that
  // runs but surfaces nothing from its response — the step-8 "prints only the
  // filename" smell — fails here.
  const leaves = step.responses.flatMap((r) => stringLeaves(r.body));
  if (leaves.length > 0) {
    const surfaced = leaves.some((leaf) => result.stdout.includes(leaf));
    if (!surfaced) {
      fail(
        label + ': ran, but no part of the intercepted response surfaced in the output.\n' +
          'Expected one of: ' + leaves.slice(0, 8).join(' | ') + '\nGot:\n' + result.stdout
      );
    }
  }
  console.log('  OK  ' + label);
}

// 3. A wrong-plane call fails as the real exception, not a wrapper.
py.FS.writeFile('__run_target__', 'import boto3\nboto3.client("bedrock-agent-runtime", region_name="eu-central-1").converse(modelId="x", messages=[], inferenceConfig={"maxTokens": 1})\n');
const wrong = JSON.parse(await py.runPythonAsync(RUN_HARNESS));
if (!wrong.traceback) {
  fail('converse on the agent client should have raised, but did not');
}
console.log('  wrong-plane call raises a real exception OK');

// 4. A configuration step has no responses. The Rust side omits the empty
// array, so boot() receives undefined and must default it. Installing the
// intercept with undefined must not raise (the `undefined`-token NameError
// bug), and a call then fails honestly as EndpointConnectionError rather than
// returning a mocked body.
await py.runPythonAsync(buildIntercept(undefined));
py.FS.writeFile('__run_target__', 'import boto3\ntry:\n    boto3.client("bedrock-runtime", region_name="eu-central-1").converse(modelId="x", messages=[{"role":"user","content":[{"text":"hi"}]}], inferenceConfig={"maxTokens":1})\n    print("NO_ERROR")\nexcept Exception as e:\n    print(type(e).__name__)\n');
const config = JSON.parse(await py.runPythonAsync(RUN_HARNESS));
if (config.traceback) {
  fail('config-step intercept install raised:\n' + config.traceback);
}
if (!config.stdout.includes('EndpointConnectionError')) {
  fail('a config-step call should fail as EndpointConnectionError; got: ' + config.stdout);
}
console.log('  config step (no responses) installs and fails calls honestly OK');

console.log('\nSMOKE TEST PASSED');
