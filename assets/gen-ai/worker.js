// The GenAI lab's Pyodide worker.
//
// Runs in its own thread so a visitor's `while True` can be terminated without
// taking the page down. Served as a content-hashed asset from the lab origin
// (not a Blob), which is why `worker-src 'self'` suffices and no `blob:` is
// needed, and why this is plain JavaScript rather than a string embedded in
// workbench.js. The embedded-string form caused repeated backtick-escaping
// breakage; a real file has none of that.
//
// Browser-verified via the Pyodide reproduction harness in
// crates/gen-ai-core/tests (node), and in the browser against the
// phase2/browser-verify.md checklist.

'use strict';

let pyodide = null;

// The transport intercept, as Python. Replaces
// botocore.httpsession.URLLib3Session.send so boto3 validates and signs the
// call for real, then the authored response is returned in place of the
// network hop. The `__RESPONSES_JSON__` marker is replaced by the main thread
// with the step's responses before this source runs; see boot().
const INTERCEPT = `
import os
import json
import io
from botocore.awsrequest import AWSResponse
import botocore.httpsession
from urllib.parse import urlparse

# Dummy credentials. botocore signs every request and raises
# NoCredentialsError without them; a browser has no credential chain and no
# IMDS to fall back to. Never sent anywhere real because the transport is
# intercepted, but signing must complete for the request to reach the seam.
os.environ.setdefault("AWS_ACCESS_KEY_ID", "lab")
os.environ.setdefault("AWS_SECRET_ACCESS_KEY", "lab")
os.environ.setdefault("AWS_DEFAULT_REGION", "eu-central-1")

_RESPONSES = json.loads(__RESPONSES_JSON__)


class _RawBody:
    # botocore reads a response body via raw.stream(); a bare BytesIO has no
    # such method. This mimics the urllib3 response just enough: stream()
    # yields the bytes once, read() returns them.
    def __init__(self, data):
        self._data = data
        self._consumed = False

    def stream(self, amt=None, decode_content=None):
        if self._consumed:
            return
        self._consumed = True
        yield self._data

    def read(self, amt=None):
        if self._consumed:
            return b""
        self._consumed = True
        return self._data

    def close(self):
        pass


def _service_from_request(request):
    # The service is the first host label, e.g.
    # bedrock-agent-runtime.eu-central-1.amazonaws.com. The transport-layer
    # context carries no service_id, so the URL is the reliable source for it.
    host = urlparse(request.url).hostname or ""
    return host.split(".")[0]


# Capture the operation name botocore itself resolved, threaded to the send
# seam through the request context.
#
# The URL cannot name the operation on its own. Most rest-json operations put
# their verb in the trailing path segment (Retrieve is
# POST /knowledgebases/{id}/retrieve), but REST collection operations do not:
# StartIngestionJob is PUT /knowledgebases/{kb}/datasources/{ds}/ingestionjobs/,
# whose trailing segment is the collection ingestionjobs, not the operation.
# Guessing from (method, path) means a lookup table that grows with every
# operation of an unusual shape and is wrong by default.
#
# botocore already knows the operation. The before-call event fires with the
# resolved operation model, and the context dict it receives is the same one
# the request carries at send time. Registering the hook on every created
# client and stashing the operation name there lets the send seam match on the
# name botocore chose -- correct for every service, no per-operation table.
def _lab_before_call(params, **kwargs):
    model = kwargs.get("model")
    ctx = params.get("context")
    if ctx is not None and model is not None:
        ctx["lab_operation"] = getattr(model, "name", None)


import botocore.client

# Guard against double-wrapping. The intercept source is re-run on every step
# mount (each step installs its own responses), and wrapping create_client a
# second time would capture the already-wrapped function as the original and
# recurse forever. Stash the genuine original on the class the first time and
# reuse it on every subsequent install.
if not hasattr(botocore.client.ClientCreator, "_lab_orig_create_client"):
    botocore.client.ClientCreator._lab_orig_create_client = (
        botocore.client.ClientCreator.create_client
    )

_lab_orig_create_client = botocore.client.ClientCreator._lab_orig_create_client


def _lab_create_client(self, *args, **kwargs):
    # Register on every client the visitor builds, from any session, so the
    # operation name is present no matter how the client was constructed.
    client = _lab_orig_create_client(self, *args, **kwargs)
    client.meta.events.register("before-call", _lab_before_call)
    return client


botocore.client.ClientCreator.create_client = _lab_create_client


def _operation_from_request(request):
    # The operation name botocore resolved, stashed by _lab_before_call. The
    # trailing-path-segment fallback covers the (unexpected) case of the hook
    # not having run, so a verb-in-path operation still matches.
    ctx = request.context or {}
    op = ctx.get("lab_operation")
    if op:
        return op
    path = urlparse(request.url).path or ""
    segments = [s for s in path.split("/") if s]
    return segments[-1] if segments else ""


def _intercepted_send(self, request):
    service = _service_from_request(request)
    action = _operation_from_request(request).lower()

    body = None
    for r in _RESPONSES:
        if r["service"] == service and r["operation"].lower() == action:
            body = r["body"]
            break

    if body is None:
        # A call the step did not author. Fail honestly rather than succeed
        # against nothing.
        from botocore.exceptions import EndpointConnectionError
        raise EndpointConnectionError(endpoint_url=request.url)

    payload = json.dumps(body).encode("utf-8")
    return AWSResponse(
        url=request.url,
        status_code=200,
        headers={"content-type": "application/x-amz-json-1.1"},
        raw=_RawBody(payload),
    )


botocore.httpsession.URLLib3Session.send = _intercepted_send
`;

// Jedi completion setup. Runs once at boot; reads the live boto3, so
// completions track whatever botocore is pinned with no list to maintain.
const COMPLETION_SETUP = `
import jedi
import boto3

_LAB_NS = {"boto3": boto3}


def _lab_complete(source, line, column):
    # Interpreter, not Script: completes against live objects, so a client's
    # operation methods are real. Line is 1-based for jedi; editor sends 0.
    try:
        script = jedi.Interpreter(source, [_LAB_NS])
        completions = script.complete(line + 1, column)
    except Exception:
        return []
    out = []
    for c in completions[:40]:
        out.append({
            "name": c.name,
            "complete": c.complete,
            "type": c.type,
        })
    return out
`;

// Runs the visitor's file and captures stdout + the traceback in Python.
// Returned as JSON so the main thread gets the real traceback every time,
// rather than a PythonError whose JS message can arrive empty.
const RUN_HARNESS = `
import sys, io, json, traceback
_out = io.StringIO()
_old = sys.stdout
sys.stdout = _out
_tb = None
try:
    with open("__run_target__") as _f:
        _src = _f.read()
    # Friendly filename so the traceback reads "<your code>", line N.
    exec(compile(_src, "<your code>", "exec"), {"__name__": "__main__"})
except BaseException:
    _tb = traceback.format_exc()
finally:
    sys.stdout = _old
json.dumps({"stdout": _out.getvalue(), "traceback": _tb})
`;

async function boot(base, files, responsesJson, packages, wheels) {
  importScripts(base + 'pyodide.js');
  pyodide = await loadPyodide({ indexURL: base });

  if (wheels && wheels.length) {
    // ssl first, by distribution name: botocore -> urllib3 -> import ssl, and
    // a browser Python has none. Pyodide's wasm ssl (and openssl) satisfy it.
    await pyodide.loadPackage('ssl');
    // Then the wheels by explicit URL. loadPackage, not micropip: micropip
    // re-resolves each wheel's declared deps even with deps=False and rejects
    // boto3/botocore; loadPackage installs URLs with no resolution, which is
    // right because the whole closure is vendored.
    await pyodide.loadPackage(wheels.map((w) => base + 'wheels/' + w));
  } else {
    await pyodide.loadPackage('micropip');
    const micropip = pyodide.pyimport('micropip');
    await micropip.install(packages, { index_urls: [base + 'wheels/'] });
  }

  for (const f of files) {
    const dir = f.path.includes('/') ? f.path.replace(/\/[^/]*$/, '') : '';
    if (dir) pyodide.FS.mkdirTree(dir);
    pyodide.FS.writeFile(f.path, f.body);
  }

  // Install the intercept with the step's responses substituted in. The
  // intercept does json.loads(__RESPONSES_JSON__), so the substituted text
  // must be a *string literal* containing JSON, not a bare array literal.
  // That means double-encoding: the inner stringify produces the JSON, the
  // outer wraps it in quotes. A single stringify yields `[{...}]`, which
  // Python reads as a list and json.loads rejects with "must be str, not
  // list". The smoke test double-encodes the same way.
  // A configuration step has no responses, and the Rust side omits an empty
  // array entirely (skip_serializing_if), so responsesJson arrives undefined.
  // Default it to [] — JSON.stringify(undefined) is the string "undefined",
  // which would substitute a bare `undefined` token into the Python and raise
  // NameError. An empty list is the correct intercept for a config step:
  // nothing to answer, any call fails as a real EndpointConnectionError.
  const responses = responsesJson ?? [];
  const intercept = INTERCEPT.replace(
    '__RESPONSES_JSON__',
    JSON.stringify(JSON.stringify(responses)),
  );
  await pyodide.runPythonAsync(intercept);
  await pyodide.runPythonAsync(COMPLETION_SETUP);
}

async function run(path) {
  pyodide.FS.writeFile('__run_target__', pyodide.FS.readFile(path), { encoding: 'binary' });
  const result = await pyodide.runPythonAsync(RUN_HARNESS);
  const parsed = JSON.parse(result);
  if (parsed.traceback) {
    console.error('[workbench] run error:\n' + parsed.traceback);
    postMessage({ kind: 'traceback', text: parsed.traceback });
  }
  postMessage({ kind: 'output', stdout: parsed.stdout, stderr: '', errored: Boolean(parsed.traceback) });
}

onmessage = async (e) => {
  const m = e.data;
  try {
    if (m.kind === 'boot') {
      await boot(m.base, m.files, m.responses, m.packages, m.wheels);
      postMessage({ kind: 'ready' });
    } else if (m.kind === 'run') {
      await run(m.path);
    } else if (m.kind === 'write') {
      pyodide.FS.writeFile(m.path, m.body);
      postMessage({ kind: 'written', path: m.path });
    } else if (m.kind === 'complete') {
      if (!pyodide) {
        postMessage({ kind: 'completions', seq: m.seq, items: [] });
        return;
      }
      const complete = pyodide.globals.get('_lab_complete');
      let items = [];
      try {
        const result = complete(m.source, m.line, m.column);
        items = result.toJs({ dict_converter: Object.fromEntries });
        result.destroy();
      } catch {
        items = [];
      } finally {
        complete.destroy();
      }
      postMessage({ kind: 'completions', seq: m.seq, items });
    }
  } catch (err) {
    postMessage({ kind: 'fatal', text: String((err && err.stack) || err) });
  }
};
