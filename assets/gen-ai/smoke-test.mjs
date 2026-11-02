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
import { readFile } from 'node:fs/promises';
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

// The authored scenario's assembled call and responses, mirroring
// content/gen-ai/policy-assistant.json. If the scenario changes materially,
// this harness is updated with it — it is a smoke test of the mechanism, not
// a content test.
const RESPONSES = [
  { service: 'bedrock-agent-runtime', operation: 'Retrieve', body: { retrievalResults: [{ content: { text: 'Parental leave in Germany: up to 156 weeks.' }, location: { type: 'S3', s3Location: { uri: 's3://policy-corpus-eu/hr/parental-leave-de-v7.pdf' } }, score: 0.82 }] } },
  { service: 'bedrock-runtime', operation: 'Converse', body: { output: { message: { role: 'assistant', content: [{ text: 'Up to 156 weeks. Source: hr/parental-leave-de-v7.pdf.' }] } }, stopReason: 'end_turn', usage: { inputTokens: 412, outputTokens: 58, totalTokens: 470 } } },
];

const SCENARIO = `
import boto3
agent = boto3.client("bedrock-agent-runtime", region_name="eu-central-1")
runtime = boto3.client("bedrock-runtime", region_name="eu-central-1")
QUESTION = "How many weeks of parental leave apply in Germany?"
found = agent.retrieve(knowledgeBaseId="KB1234567890", retrievalQuery={"text": QUESTION})
for chunk in found["retrievalResults"]:
    print(chunk["location"]["s3Location"]["uri"])
context = "\\n\\n".join(c["content"]["text"] for c in found["retrievalResults"])
reply = runtime.converse(
    modelId="eu.anthropic.claude-sonnet-4-5-20250929-v1:0",
    messages=[{"role": "user", "content": [{"text": context}]}],
    inferenceConfig={"maxTokens": 512},
)
print(reply["stopReason"])
print(reply["output"]["message"]["content"][0]["text"])
`;

console.log('Booting Pyodide from vendored runtime…');
const py = await loadPyodide({ indexURL: pyodideDir + '/' });

console.log('Loading ssl + ' + WHEELS.length + ' wheels…');
await py.loadPackage('ssl');
await py.loadPackage(WHEELS.map((w) => join(wheelDir, w)));

console.log('Installing intercept and completion setup (from worker.js)…');
// This must mirror boot() in worker.js exactly — same double-encoding. The
// intercept does json.loads(__RESPONSES_JSON__), so the marker is replaced
// with a string literal containing JSON (double stringify), not a bare array
// (single stringify), which Python reads as a list and rejects. Keeping this
// identical to boot() is the whole point: a mismatch here is the bug the
// smoke test exists to catch.
const interceptForSmoke = INTERCEPT.replace(
  '__RESPONSES_JSON__',
  JSON.stringify(JSON.stringify(RESPONSES)),
);
await py.runPythonAsync(interceptForSmoke);
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

// 2. The authored call runs end to end through the intercept.
py.FS.writeFile('__run_target__', SCENARIO);
const result = JSON.parse(await py.runPythonAsync(RUN_HARNESS));
if (result.traceback) {
  fail('scenario raised:\n' + result.traceback);
}
const out = result.stdout;
for (const expected of ['parental-leave-de-v7.pdf', 'end_turn', '156 weeks']) {
  if (!out.includes(expected)) {
    fail('scenario output missing "' + expected + '". Got:\n' + out);
  }
}
console.log('  scenario OK (intercepted response parsed and printed)');

// 3. A wrong-plane call fails as the real exception, not a wrapper.
py.FS.writeFile('__run_target__', 'import boto3\nboto3.client("bedrock-agent-runtime", region_name="eu-central-1").converse(modelId="x", messages=[], inferenceConfig={"maxTokens": 1})\n');
const wrong = JSON.parse(await py.runPythonAsync(RUN_HARNESS));
if (!wrong.traceback) {
  fail('converse on the agent client should have raised, but did not');
}
console.log('  wrong-plane call raises a real exception OK');

console.log('\nSMOKE TEST PASSED');
