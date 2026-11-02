# Phase 2 — Self-hosting the runtime on AWS

Decision: the Pyodide runtime and the Python wheels are vendored onto this
project's own AWS origin, not pulled from a CDN. The workbench loads
everything from `/gen-ai/assets/pyodide/`, so the page stays under
`script-src 'self'` / `connect-src 'self'` with no third-party vendor.

## Why

The first browser run hung on "Loading the runtime…" because the workbench
asset pointed `PYODIDE_BASE` at `cdn.jsdelivr.net` while the page's CSP only
allowed `'self'`. Three things were blocked: the worker's `importScripts` to
the CDN (governed by `script-src`), Pyodide's fetches of its wasm and data
(governed by `connect-src`), and the Blob worker itself (governed by
`worker-src`).

Two ways to reconcile the asset with the policy: allow the CDN host, or
self-host. We self-host. Allowing the CDN would add `cdn.jsdelivr.net` to
`script-src` on every workbench page — a second vendor trusted to execute
script, and a runtime dependency on jsDelivr's uptime. Keeping everything on
AWS is the project's standing preference and keeps the CSP honest: `'self'`
means this origin, and now it is true.

The cost is artifact weight, not money. Egress for a one-time ~8–15 MB
gzipped first visit is pennies at this traffic, and the files are
content-addressed with an immutable TTL, so returning visitors and warm edges
pay nothing. The real tradeoff is that the runtime is too large to embed in
the Lambda binary, so it lives in an S3 bucket behind the existing CloudFront
behavior. That reopens the lab's "one binary, no buckets" line, consciously:
a multi-megabyte binary payload belongs in object storage, which is exactly
what the serverless standards say for large static assets (S3 + OAC +
CloudFront, versioning on, public access blocked).

## What gets vendored

Everything the worker loads, under `/gen-ai/assets/pyodide/`:

- The Pyodide core for the pinned `PYODIDE_VERSION` (`v0.26.4`): `pyodide.js`,
  `pyodide.asm.js`, `pyodide.asm.wasm`, `python_stdlib.zip`,
  `pyodide-lock.json`, and the repodata the loader reads. Take the official
  `full` distribution for that version and keep the files the loader asks for.
- The wheels for `boto3`, `botocore`, `jedi`, and their full transitive
  closure, under `/gen-ai/assets/pyodide/wheels/`. At time of writing that set
  is approximately: `boto3`, `botocore`, `jmespath`, `s3transfer`, `urllib3`,
  `python-dateutil`, `six`, `jedi`, `parso`. The authoritative list is
  whatever micropip actually resolves — see the pinning step.

## Vendoring, in one command

`assets/gen-ai/vendor-pyodide.mjs` does both the runtime fetch and the wheel
resolve, so the two steps that need a live network are one command run once
per Pyodide or dependency bump:

```
node assets/gen-ai/vendor-pyodide.mjs
```

It downloads the pinned Pyodide `full` core into `assets/gen-ai/pyodide/`,
then boots that Pyodide under Node, runs the real micropip resolve for
`boto3` + `botocore` + `jedi`, downloads every wheel it fetches into
`assets/gen-ai/pyodide/wheels/`, and prints the exact `WHEELS` array to paste
into `workbench.js`. The filename list is observed from a genuine resolve, not
guessed, which is the only way it stays correct across releases.

With `WHEELS` populated, `boot()` installs each wheel by same-origin URL and
never reaches PyPI again. Until it is populated, `boot()` falls back to
treating `PYODIDE_BASE + "wheels/"` as an index.

## What is committed, and what is fetched

The multi-megabyte binaries are gitignored; the commit still fully determines
the deploy. This mirrors `package-lock.json` plus `node_modules`:

- Committed: the pinned `PYODIDE_VERSION` and the `WHEELS` list in
  `workbench.js`, and `pyodide-lock.json` (the text record of exactly which
  binaries resolve). These make the result reproducible and reviewable.
- Not committed: the Pyodide core files and the `.whl` binaries under
  `assets/gen-ai/pyodide/`, which are too large for git.
- CI runs `vendor-pyodide.mjs` before `cdk synth`, so the
  `BucketDeployment` finds the files on disk and the deploy is reproducible
  from a commit without the bytes living in git. A deploy from a clean
  checkout that skips the vendor step uploads an empty bucket, and the
  workbench hangs honestly rather than the stack failing.

This keeps git-as-truth: the lockfile and the pins are the source of truth for
what ships, and the bytes are a derived cache rebuildable from them.

## Infrastructure (to add in `infra/lib/gen-ai-lab.ts`)

- An S3 bucket for the lab's large assets: versioning on, public access
  blocked, SSE enabled, tagged `project`/`stage`/`component`/`managed-by`, name
  including the stage.
- A CloudFront origin for that bucket with OAC, and the existing
  `/gen-ai/assets/*` behavior pointed at it with the immutable long-TTL cache
  policy. This behavior already lists ahead of `/gen-ai/*`, which is the
  ordering the runtime fetch depends on; confirm it still does after the
  origin change.
- A lifecycle rule expiring noncurrent versions, per the storage standards.
- The vendored files uploaded as part of deploy (a `BucketDeployment` or the
  publish pipeline), content-hashed, so a version bump is a new key and the
  immutable cache is never stale.

The small hand-written scripts (`workbench.js`, `nav.js`) stay embedded in the
Lambda and served from it, unchanged. Only the large binary runtime moves to
the bucket.

## CSP

The workbench policy is now `script-src 'self' 'wasm-unsafe-eval'`,
`connect-src 'self'`, `worker-src 'self' blob:`. No CDN host. `blob:` is on
`worker-src` only, because the runtime thread is spawned from a Blob worker
whose body ships inside `workbench.js`; the blob inherits the document policy,
so the worker is no more powerful than the page. The overview and landing keep
the style-only policy, as tight as the resume's.

## Running boto3 under Pyodide: what it actually takes

Getting the authored call to run end to end surfaced a chain of requirements
that are not obvious from the design and are worth recording, because every
future scenario that calls an AWS service inherits them. Each was found by
reproducing the failure under Pyodide and reading the real traceback, not by
guessing.

- **The ssl module.** botocore imports urllib3, which imports `ssl`. A browser
  Python has no `ssl` (it needs real sockets). Pyodide ships a wasm `ssl`
  package depending on `openssl`; the worker must `loadPackage("ssl")` before
  the wheels, and both zips must be vendored into the core dir. Symptom
  without it: `ImportError: cannot import name 'ssl' from urllib3.util.ssl_`.

- **Dummy credentials.** botocore signs every request and raises
  `NoCredentialsError` without a credential chain; in a browser there is none,
  and it even probes the EC2 metadata endpoint looking for one. The intercept
  sets throwaway `AWS_*` env vars so signing completes and the request reaches
  the transport seam. They are never sent anywhere real.

- **Install with no dependency resolution.** micropip insists on resolving
  each wheel's declared dependencies even with `deps=False`, and rejects
  boto3/botocore as "not a pure Python wheel" before fetching anything. The
  fix is `pyodide.loadPackage(urls)`, which installs explicit wheel URLs with
  no resolution — correct here because the whole transitive closure is
  vendored. The complete closure matters: nothing resolves the gaps.

- **Keying the intercept off the URL, not the context.** At the transport
  layer the request context does not carry `operation_name` or `service_id`,
  and Bedrock's rest-json protocol sends no `X-Amz-Target` header. The
  reliable source is the URL: service is the first host label
  (`bedrock-agent-runtime`), operation is the trailing path segment
  (`/retrieve` → Retrieve). Symptom of keying off context: every call misses
  and raises `EndpointConnectionError`.

- **The response body needs a `stream()`.** botocore reads a response via
  `raw.stream()`. A bare `io.BytesIO` has no such method. The intercept wraps
  the payload in a minimal `_RawBody` that yields the bytes from `stream()`
  (and `read()`), mimicking the urllib3 response botocore expects. Symptom:
  `AttributeError: '_io.BytesIO' object has no attribute 'stream'`.

- **Content must pass real validation.** Because the genuine library validates,
  authored snippets must use values that pass. The first `knowledgeBaseId` was
  8 characters; botocore's minimum is 10. That `ParamValidationError` was the
  library working as intended — the lesson, not a bug — but an authored
  example that is meant to *succeed* has to clear validation.

All of these live in the intercept in `assets/gen-ai/worker.js` and are
exercised by the smoke test described under Testing below.

## Testing

`assets/gen-ai/smoke-test.mjs` is the test the Rust side cannot be. It boots
the vendored Pyodide under Node, applies the real `worker.js` intercept and
completion setup (extracted from the source, not copied, so it cannot drift),
and asserts three things end to end:

- the authored call runs through the intercept and the intercepted response is
  parsed and printed,
- completion resolves `boto3.` against the live library,
- a wrong-plane call raises a real exception rather than a wrapper.

It exits non-zero on any failure. CI runs it in the check job after vendoring,
so a broken worker blocks a pull request rather than surfacing in a browser.
Every runtime bug this phase hit — ssl, credentials, the install path, the
URL-keyed intercept, the response `stream()`, the backtick-escaping breakage —
is caught by this harness in seconds. The worker being a served file rather
than an inline string is what makes the escaping class of bug impossible now;
the smoke test covers the rest.

## Status

- [x] Worker `worker-src 'self' blob:` fix, so the Blob worker can spawn.
- [x] `PYODIDE_BASE` points at the self-hosted path; `boot()` installs wheels
      from same-origin URLs.
- [x] S3 asset bucket + CloudFront origin (OAC, versioning, SSE, blocked
      public access, lifecycle, immutable TTL) in `gen-ai-lab.ts` and
      `site-stack.ts`, behaviors ordered pyodide → assets → catch-all.
- [x] `vendor-pyodide.mjs` fetches the core, the ssl/openssl distribution
      packages, and resolves the wheels in one command; binaries gitignored,
      lockfile kept.
- [x] Ran `vendor-pyodide.mjs`; the 9 wheels and ssl/openssl are vendored and
      the `WHEELS` list is pinned in `workbench.js`.
- [x] The authored call runs end to end locally: the intercept returns the
      authored response and botocore parses it. Verified in a browser — the
      S3 URI, `end_turn`, and the cited answer print. Typeahead works.
- [x] Runtime integration fixes (ssl, credentials, loadPackage, URL-keyed
      intercept, `_RawBody`, KB-id length) — see the section above.
- [x] Wire the vendor step into CI before `cdk synth` (deploy job).
- [ ] Run the remaining browser-verify checklist items (the break-it cases,
      swap/navigation, reload reseed) and confirm no request leaves the lab
      origin.
