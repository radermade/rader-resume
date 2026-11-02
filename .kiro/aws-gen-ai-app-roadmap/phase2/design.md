# Phase 2 — Design

## Where the work happens

The division is the whole design:

| Layer | Owns |
| --- | --- |
| `gen-ai-core` | The scenario model, the seed files, the intercepted responses, the markup. Runs nothing |
| `assets/gen-ai/*.js` | Mounting the runtime, the file tree, the editor, the output pane, the intercept |
| The browser | Executing Python |
| The Lambda | Serving documents. Nothing else |

No run endpoint. A command costs zero invocations, there is no public POST
doing work, and the submitted text never leaves the tab. That is a smaller
surface than the server-side checker this phase replaced, not a larger one.

## The step's workbench block

```rust
pub struct CallStep {
    pub title: String,
    pub decision: String,
    pub why: String,
    /// The pieces of the call, each labelled. Shown in the walkthrough for
    /// the visitor to assemble in the editor. The step does not ship the
    /// finished file.
    pub snippets: Vec<Snippet>,
    pub infrastructure: String,
    pub workbench: Workbench,
}

pub struct Snippet {
    /// What this piece does, e.g. "Retrieve the policy chunks".
    pub label: String,
    /// `python` or `shell`. Labels the block; does not drive execution.
    pub language: String,
    pub source: String,
}

pub struct Workbench {
    /// Seeded into the filesystem when the step mounts. A scaffold, not the
    /// answer: a stub, imports, reference notes. The visitor assembles the
    /// snippets into the open file. The starting state is deliberately not a
    /// working call, so the lesson is what happens when you write one.
    pub files: Vec<SeedFile>,
    /// Which file the editor opens on. The one the visitor writes into.
    pub open: String,
    /// What the intercepted transport returns, keyed by the operation it
    /// answers.
    pub responses: Vec<Intercepted>,
    /// Prose the lesson adds beside a genuine error, for when the real
    /// message is correct but unhelpful. Never replaces the error.
    pub reasons: Vec<Reason>,
}

pub struct SeedFile {
    pub path: String,
    pub body: String,
}

pub struct Intercepted {
    /// botocore service id, e.g. `bedrock-agent-runtime`.
    pub service: String,
    /// botocore operation name, e.g. `Retrieve`.
    pub operation: String,
    /// Returned as the parsed response body.
    pub body: serde_json::Value,
}

pub struct Reason {
    /// Matched against the exception class name botocore raised.
    pub on_exception: String,
    /// Optional narrowing, matched against the message.
    pub when_message_contains: Option<String>,
    pub prose: String,
}
```

## Why snippets rather than a seeded answer

The editor starts without the completed call. The walkthrough shows the
pieces as labelled snippets, and assembling them into the open file is the
exercise. This is the correction that shaped the phase: a seeded working file
with a Run button teaches that you can press Run. It does not teach the call,
because the call is already written. You learn the request shape by composing
it, so the shape is what the visitor supplies and the scaffold is what the
step supplies.

That puts three constraints on authored content, each a build check:

- A call step has at least one snippet, or there is nothing to assemble.
- The open seed file is a scaffold, not the answer. If it already contained
  every operation the step teaches, the editor would start finished. The
  check rejects a seed where every intercepted operation is already called.
- Every intercepted response is reachable from a snippet. A response the
  walkthrough never teaches the visitor to write is content nobody reaches.

The snippets are also the teaching content without the runtime. They are
server-rendered in the walkthrough, HTML-escaped by `maud`, so the step reads
correctly before Pyodide loads and with script disabled entirely. The runtime
turns reading into doing; it is not a prerequisite for the lesson.

`Reason` keys on the exception botocore actually raised, which is the
important inversion. The earlier draft matched authored strings against the
visitor's text and had to guess what they got wrong. Here the library has
already decided, precisely, and the lesson only supplies the sentence the
error cannot: which product the step uses and why the other one loses.

A `Reason` that matches nothing is harmless. A genuine error with no reason
beside it still teaches, because it is the real error.

## Intercepting botocore

botocore is pure Python and installs under Pyodide via micropip. It builds
clients, validates parameters, and serializes requests correctly, then fails
at transport because the runtime has no sockets.

The seam, confirmed rather than assumed: botocore sends through
`botocore.httpsession.URLLib3Session.send`, which takes an
`AWSPreparedRequest` and returns an `AWSResponse`. That is the single method
the intercept replaces. It is the documented transport path, not an internal
endpoint object, which is what makes it safe to patch. Everything above it is
the genuine library:

- `ParamValidationError` for a missing required key or a bad shape, raised
  during serialization, before `send` is ever reached. The intercept is not
  involved, which is exactly why a missing `maxTokens` teaches: the real
  library rejects it on its own.
- `ClientError` and its subclasses, when the intercept returns an
  `AWSResponse` with a non-2xx status and an error body. botocore parses that
  into the real exception class from the error code, so the lab does not
  choose the class, botocore does.
- `EndpointConnectionError` left reachable for a service the step did not
  author, so an unanticipated call fails honestly rather than succeeding
  silently against nothing.

Pin the botocore version in the asset. The lab teaches request shape, not
botocore internals, so a moving transport API is pure downside and a pinned
library is a feature. The version to pin is whatever the browser spike
confirms installs cleanly under the target Pyodide release.

The one thing still owed to a browser, because it cannot be reached from
Rust: that `URLLib3Session.send` is still the seam in the pinned version, and
that a replaced `send` returning a constructed `AWSResponse` produces a real
parsed response above it. The intercept is written against that contract; the
spike confirms the contract holds.

## The `aws` shim

One line of input, mapped onto the same intercepted client. `aws bedrock-agent-runtime retrieve --knowledge-base-id X` becomes the
equivalent `boto3` call, so both forms reach one code path and one set of
authored responses.

Deliberately a shim, not real `awscli`. The real package is pure Python but
heavy and pulls docutils, and the exam tests the *shape* of a CLI call rather
than the CLI's own argument parser. Whether it installs is open question 2 in
the roadmap and worth a spike, but the design does not depend on the answer.

Argv translation is mechanical: `--kebab-case` to `PascalCase` keys, service
and operation from the first two tokens. A flag the shim does not understand
is reported as unrecognised rather than dropped, because silently ignoring an
argument is how a visitor concludes their wrong call was right.

## Layout

```
.lab-step            grid; one column on the overview, two from step 2
  .lab-lesson        swapped on Next
  .lab-workbench     mounted once, survives Next
    .lab-files       the tree
    .lab-editor      textarea plus a line-number gutter
    .lab-output      stdout, stderr, traceback
    .lab-cli         one line
```

The lesson column carries a "Write the call" pane: the labelled snippets, with
the instruction to assemble them in the editor and Run. Reading them is the
prompt; the editor to their right is where the practice happens.

The workbench mounts once and is not inside the swap target. That is what
keeps the files when the lesson changes, and it is why the two-column layout
arrives now rather than being retrofitted.

Without script there is no live workbench: the lesson renders, the labelled
snippets are readable in the walkthrough, and a `noscript` fallback shows the
scaffold the editor would open on. The step still teaches, because the
snippets are the teaching content and they are server-rendered. That is a
real degradation rather than a pretence, and it is stated rather than hidden.

## The editor

A textarea with a synced line-number gutter, not a library. Tracebacks name
lines, so the gutter is function. A real editor component is small next to the
runtime and is the upgrade if the gutter chafes, but starting without one
keeps the asset count down while the harder parts settle.

Tab inserts spaces. That is the one editing affordance worth special-casing,
because Python.

## Policy

Pyodide requires `script-src 'self'` and `wasm-unsafe-eval`. The workbench
also runs its runtime in a Blob worker, so `worker-src` is `'self' blob:`.
All three are confined to call-step pages, so the resume's policy and the
lab's own overview and landing are untouched.

Self-hosted, not from a CDN. An earlier draft of this phase pulled Pyodide
from jsDelivr "so the binary stays small", which contradicted the `'self'`
policy the same section promised: the first browser run hung because the CDN
was blocked. The runtime and the wheels are vendored onto this origin instead
and served from the lab's asset bucket, so `script-src 'self'` and
`connect-src 'self'` are literally true and the lab stays on one vendor. The
tradeoff — the runtime is too large to embed, so it lives in S3 rather than
the Lambda binary — is recorded in `self-hosting.md`. Keeping to AWS was the
deciding factor over a one-line CDN URL.

Worth stating plainly: `wasm-unsafe-eval` weakens the defence against
*injected* script, not against the visitor's own code, which is already theirs
to run in their own tab. The mitigation is that the lab stores nothing and
echoes nothing, so there is no stored-XSS path to combine it with. `blob:` is
on `worker-src` only; the Blob worker inherits the document policy, so it is
no more powerful than the page that spawned it.

One consequence: the argument for hand-rolling the lesson swap to keep
`script-src` at a single hash is void from this phase on. That battle is lost
to Pyodide, so a library for the swap is now free on the policy axis and the
choice can be made on its own merits.

## Runaway code

A visitor will paste an infinite loop. Pyodide runs on the main thread by
default, which means a hung tab and a site that looks broken.

The answer is to run Python in a web worker, so a runaway can be terminated
without taking the page with it. That is a structural decision rather than a
tweak, which is why it belongs in this phase and not after the first report.

The worker is a served, content-hashed file (`worker.js`), not a Blob built
from a string inside `workbench.js`. The inline-string form was a standing
source of bugs: the worker body was one giant template literal, and any
backtick inside it — even in a comment — silently closed the string and turned
Python into live JS, which `node --check` did not reliably catch. A real file
has no escaping. It also lets `worker-src` stay `'self'` with no `blob:`, since
the worker loads from this origin. The one value that crosses from Rust to the
browser is the worker's URL, carried in the inert data block beside the
workbench.

## Budget

The lab takes its own page-weight budget, which resolves phase 1's open
question. The runtime is megabytes and loads on first use of the workbench;
the landing and the lesson prose stay within a budget worth naming. A visitor
who reads a scenario and never runs anything should pay nothing for the
runtime.
