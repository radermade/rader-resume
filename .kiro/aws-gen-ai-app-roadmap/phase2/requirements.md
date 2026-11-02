# Phase 2 — The workbench is real

Status: not started

Goal: an environment you can work in. Real CPython in the browser, a
filesystem, an editor, and the genuine `boto3` with its transport
intercepted. One step authored against it.

Supersedes an earlier draft of this phase that specified a command checker
over a textbox. That design matched authored strings against submitted text,
which can only teach that you failed to type a token. It could not produce a
`ParamValidationError`, and those exception names are a large part of what
domain 5 tests.

The editor starts empty of the answer. The walkthrough shows the pieces of
the call as labelled snippets; the visitor assembles them in the editor and
runs them. You learn the call by writing it, not by reading a finished file
and pressing Run. The seed is a scaffold — a stub, imports, reference notes —
never the completed call.

## The sandbox

- R2.1 Python is real CPython compiled to WebAssembly, running in the
  visitor's tab. Not a simulation and not a matcher.
- R2.2 Nothing executes on the Lambda and nothing reaches AWS. There is no
  account, no model call, no credentials in a browser, and no run endpoint.
- R2.3 `boto3` and `botocore` are the genuine libraries, installed from PyPI
  into the runtime. Parameter validation and exception classes are theirs.
- R2.4 Only the transport is intercepted. A call that would reach the network
  returns the step's authored response instead.
- R2.5 A wrong call fails as the real exception. A missing `maxTokens` raises
  `ParamValidationError` from botocore, not a string the lesson wrote.
- R2.6 The lesson may add a reason beside a genuine error, for when the real
  message is correct but unhelpful. The reason never replaces the error.
- R2.7 The runtime loads on first use of the workbench, not on page load.
- R2.8 The filesystem is in-memory, seeded per step, and resets on reload.
- R2.9 Nothing the visitor writes is stored, transmitted, or logged.

## The workbench

- R2.10 A file tree, an editor, and an output pane, mounted as a column beside
  the lesson from step 2.
- R2.11 Not a shell. A file tree replaces `cd` and `ls`, which are not on the
  exam and would cost a dozen builtins of edge cases.
- R2.12 The editor shows line numbers. A traceback names a line and the
  teaching loop is read-then-fix, so this is function rather than decoration.
- R2.13 Running a file executes it under the real runtime and shows stdout,
  stderr, and the traceback.
- R2.14 One line accepts an `aws` invocation, mapped onto the same intercepted
  client so the CLI form and the Python form reach one code path.
- R2.15 The overview renders no workbench element at all, not an empty one
  hidden by CSS. That follows from `Step::as_call`.
- R2.16 Advancing a step leaves the workbench mounted and its files intact.
- R2.17 A visitor's runaway loop is bounded, so an accidental `while True`
  does not look like the site is broken.

## Authored content

- R2.18 A call step carries the labelled snippets shown in the walkthrough,
  the files that seed the workbench, the intercepted responses keyed by the
  call they answer, and any reasons the lesson adds.
- R2.19 A call step with no seed files fails the build. A step with a
  workbench and nothing in it is a promise it does not keep. A call step with
  no snippets fails the build too: there would be nothing to assemble.
- R2.20 The seed is a scaffold, not the answer. The editor opens on a file the
  visitor writes into; the completed call is what they assemble from the
  snippets, not what the step ships. A build check rejects a seed that already
  contains every operation the step teaches.
- R2.21 The snippets are the teaching content, server-rendered in the
  walkthrough with a label for each piece. They reach the page with no runtime
  at all, so the step teaches before Pyodide loads and without script.
- R2.22 Every intercepted operation is reachable from a snippet. A response
  the walkthrough never teaches the visitor to call is content nobody reaches,
  and a build check rejects it.

## Constraints

- R2.23 The resume's payload, policy, and routes are untouched. Runtime and
  editor assets live on the lab behavior only.
- R2.24 The lab's policy gains `script-src 'self'` and `wasm-unsafe-eval`,
  which Pyodide requires. Recorded as a decision with the confinement above
  as its mitigation, because it weakens the defence against injected script.
- R2.25 Assets are content-hashed and long-cached. The runtime is large and
  must not be refetched per visit.
- R2.26 Still no session, no DynamoDB, no saved position, no Step Functions.
- R2.27 `gen-ai-core` still has no `aws-sdk-*` and no `lambda_*`, and the
  check that enforces it lands this phase. The workbench is the part most
  likely to tempt a heavy dependency into the crate.

## Exit criteria

The visitor assembles the call from the walkthrough's snippets into the empty
editor and runs it under real CPython. The real library validates it.
Deleting `maxTokens` raises a genuine `ParamValidationError`; a `global.`
model id and Converse on `bedrock-agent-runtime` each fail as their real
exception with the lesson's reason beside them. The overview still has no
workbench. None of it costs a Lambda invocation, and `cargo lambda watch`
serves the whole loop locally.
