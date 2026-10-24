---
inclusion: always
---

# Rust Standards

Applies to all Rust in this repo: Lambda handlers, shared crates, pipeline
tasks.

## Toolchain and workspace

- Pin the toolchain in `rust-toolchain.toml`. Stable channel, explicit version,
  `targets = ["aarch64-unknown-linux-gnu"]`.
- Single cargo workspace. Shared crate versions live in
  `[workspace.dependencies]`; member crates use `dep = { workspace = true }`.
- Build Lambda artifacts with `cargo lambda build --release --arm64`.
- Deny warnings in CI: `cargo clippy --all-targets --all-features -- -D warnings`.
- `cargo fmt --check` and `cargo deny check` (licenses, advisories) in CI.
- Forbid unsafe at the workspace root:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
```

## Crate boundaries

- `resume-core` owns the data model, validation, rendering logic, and every
  Step Functions payload type. It has no `aws-sdk-*` or `lambda_*` dependency.
  That constraint is what keeps it unit-testable and what would let it compile
  to WASM if a browser-side renderer ever appears.
- One HTTP binary. All web routes, the public read API, and the flow endpoints
  live in it and route internally. A function per route buys nothing here and
  costs cold starts, deploy complexity, and a second place for auth to live.
- One binary per Step Functions task state. These have genuinely different IAM,
  memory, and timeout needs, and separate artifacts keep retries and failures
  attributable.
- Multiple binaries is a build-target decision, not an application-count
  decision. There is one app in one workspace.

## Error handling

- Library crates (`resume-core`, anything reusable): concrete error enums with
  `thiserror`. No `anyhow` in library public APIs.
- Binary crates (handlers): `anyhow` or `eyre` at the edge, returning
  `Result<T, lambda_runtime::Error>` from the handler.
- `unwrap`/`expect` are acceptable only during cold-start init where failing
  fast is the correct behavior. Never inside request handling.
- Let the runtime catch panics. Keep `panic = "unwind"` so one bad invocation
  fails that invocation instead of poisoning a warm sandbox.
- Errors returned to callers are typed and machine-readable. Errors in logs
  include context; neither includes secrets or PII.

## Lambda handler shape

- Construct AWS clients, config, and templates once at cold start, outside the
  handler closure. Use `LazyLock` or an explicit struct passed into
  `service_fn`.
- Handlers are thin: deserialize, delegate to `resume-core`, serialize. Business
  logic lives in testable library functions with no AWS types in their
  signatures.
- Handlers must be idempotent. Assume at-least-once invocation, always.
- Respect the remaining-time budget from the Lambda context when doing
  multi-step work; fail cleanly rather than being killed mid-write.
- Logging via `lambda_runtime::tracing` / `tracing`. JSON output, no ANSI, no
  timestamps (CloudWatch supplies them). Use spans with the request ID and,
  for pipeline tasks, the commit SHA and execution name.

## Serde and payload contracts

- Every Step Functions task input and output is a named struct in
  `resume-core`, not an ad-hoc `serde_json::Value`.
- `#[serde(rename_all = "camelCase")]` on state machine payloads to match ASL
  conventions.
- Do not use `deny_unknown_fields` on state machine payloads; orchestration
  layers add fields and strictness turns that into an outage. Do use it for
  authored content documents, where an unknown field means a typo.
- Version content schemas explicitly with a `schemaVersion` field. Migrations
  are code, tested.

## State machines

Two distinct kinds. Keep them separate in your head and in the code.

In-process state (within one function):

- Model states as an enum with explicit transition functions. Make illegal
  transitions unrepresentable rather than guarded by runtime checks.
- Use the typestate pattern when a sequence must be enforced at compile time
  (e.g. `Draft` → `Validated` → `Publishable`, where only `Publishable`
  exposes `publish()`). Consuming `self` on transitions prevents reuse of a
  stale state.
- Pure transition functions, no I/O. This is where the unit tests live.

Distributed state (Step Functions):

- One Lambda per state, each a single transition: input state → output state.
- No hidden state between tasks. Everything the next task needs is in the
  payload or behind a pointer in the payload.
- Keep payloads small; pass S3 keys instead of documents.
- Tasks never assume they are the first attempt.
- There is no Rust DSL for authoring workflows. The definition is ASL in the
  IaC layer; Rust owns the payload types and the task logic, nothing else.

Rust touches Step Functions in exactly two directions:

- Inbound, as a task. The handler receives JSON and returns JSON. It must not
  know or care that a state machine invoked it, which is what keeps it callable
  from a unit test with a constructed struct.
- Outbound, as a driver, via `aws-sdk-sfn`: `start_execution`,
  `send_task_success`, `send_task_failure`, `describe_execution`,
  `start_sync_execution`. Only the HTTP handler does this.

## Task tokens

For `.waitForTaskToken` callback steps:

- A token is a bearer credential for resuming a paused workflow. It stays
  server-side, stored in the session record. It is never rendered into HTML,
  returned in a response body, or placed in a URL.
- A token is single-use. `send_task_success` succeeds once and then returns
  `TaskDoesNotExist`. A double-submitted form must be absorbed by the handler,
  not by the service: check the session record's status before calling, and
  treat an already-advanced session as success.
- Pair every `send_task_success` with the session update in a defined order.
  Advance the workflow first, then record it, so a crash between the two leaves
  a session that can be reconciled from `describe_execution` rather than a
  workflow that never resumes.
- If a task sets `HeartbeatSeconds`, something has to send heartbeats. Prefer
  not setting it over setting it and ignoring it.

## Testing

- Unit tests on pure logic in `resume-core`. These are the bulk of the suite.
- Handler tests construct the event type and call the handler function
  directly. No live AWS in unit tests; abstract AWS calls behind a trait or use
  the SDK's mocking support.
- `insta` snapshot tests for rendered HTML, JSON, and PDF metadata.
- Integration tests run against a deployed stage in CI, not against mocks.

## Performance and cold start

```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
```

- arm64 by default; it is cheaper and faster for this workload.
- Avoid pulling in a full TLS stack twice. Pick one and keep feature flags
  tight.
- Measure before optimizing. Cold start on a small Rust Lambda is already low
  double-digit milliseconds; don't trade clarity for noise.
