# Phase 2 — Tasks

Status: the core model and the worked content landed early, because
re-speccing from the console design touched those types. The dependency-ban
CI check also landed. The model was then corrected once more: the editor
starts with a scaffold, not the answer, and the walkthrough shows labelled
snippets the visitor assembles. `Snippet`, `CallStep.snippets`, the "Write the
call" pane, and the scaffold-not-answer build check are that correction, all
landed and tested. Everything browser-side — the runtime, the editor, the
intercept, the worker — is code-complete but browser-unverified and is the
bulk of what remains. `[~]` marks an item reframed from how it was first
written, with the reason inline.

## Spikes, before the design hardens

- [ ] Confirm `boto3` and `botocore` install under Pyodide via micropip, and
      find the transport seam that the intercept patches. Pin the version in
      the asset rather than chasing it across releases: the lab teaches
      request shape, not botocore internals.
- [x] Confirmed a genuine `ParamValidationError` is raised before transport,
      without the intercept involved — verified against real botocore:
      dropping the required `modelId` on Converse fails during validation.
      (The original wording guessed `maxTokens`; that is optional and does not
      fail. The claim holds for a genuinely-required parameter.)
- [ ] Decide the web worker boundary. Pyodide on the main thread means a
      visitor's `while True` hangs their tab and the site looks broken.
- [ ] Optional: whether real `awscli` installs. The committed design is a
      shim, so this blocks nothing.

## Carried from phase 1

- [x] Enforce the `gen-ai-core` dependency ban in CI. Landed early, in the
      `check` job, as a `cargo tree` grep covering `resume-core` too.
      Verified passing against both crates' current trees.
- [ ] Name the lab's page-weight budget, which phase 1 left open and the
      runtime now forces.

## Core

The model landed early this phase, because re-speccing from the console design
touched these types and leaving them half-defined would have been worse than
finishing them.

- [x] `Workbench`, `SeedFile`, `Intercepted`, and `Reason` on `CallStep`,
      strict parsing, with build failures for a step that seeds no files,
      opens a file it does not seed, or intercepts no response.
- [x] `Snippet` and `CallStep.snippets`: the labelled pieces shown in the
      walkthrough. A call step with no snippets fails the build, because there
      would be nothing to assemble.
- [x] The seed is a scaffold, not the answer. A build check rejects an open
      file that already calls every operation the step teaches, and a test
      asserts the editor does not start finished. This is the corrected model:
      the visitor writes the call from the snippets rather than pressing Run on
      a seeded file.
- [x] Every intercepted operation is reachable from a snippet, checked in a
      test: a response no snippet teaches the visitor to call is unreachable
      content.
- [x] `Reason` keys on the exception class botocore raised, not on the
      visitor's text, with a test that a reason adds more than the exception
      name.
- [x] Serialize the workbench into the page as an inert
      `<script type="application/json">` block the asset reads by id, with
      `</` neutralized so authored Python cannot close the block early. Tested
      with a hostile seed file and a round-trip.
- [x] A test that snippet Python is not obviously broken (no tabs, every
      snippet labelled). Not a parser, by design: a full parse waits on the
      browser run.

## Assets

Written against the confirmed `URLLib3Session.send` seam, but not run here:
the crate cannot execute Pyodide, so each item below is code-complete and
browser-unverified. The "verify in a browser" checklist is their test.

- [x] `assets/gen-ai/workbench.js`, embedded in `gen-ai-core`, served
      content-hashed under `/gen-ai/assets/*` from the Lambda with an
      immutable cache. No bucket: option 1 from the hosting decision, verified
      serving locally with the right content type, caching, and a clean 404 on
      an old hash.
- [x] Lazy load: the entry module is linked only on a call step, with `defer`,
      and it pulls Pyodide from the CDN on first mount. A reader on the
      overview fetches none of it, asserted by a render test.
- [x] Mount `boto3` with `URLLib3Session.send` replaced, returning the step's
      authored response keyed by service and operation. Everything above the
      send is the real library, so a missing parameter is botocore's own
      `ParamValidationError`.
- [x] The `aws` shim: argv to service, operation, and `PascalCase` keys, onto
      the same intercepted client. A value with no flag, or a flag with no
      value, is reported rather than dropped.
- [x] File tree from the seeded files, the open file switchable, buffers kept
      per file.
- [x] Editor: textarea plus a synced line-number gutter, tab inserts four
      spaces.
- [x] Typeahead completions from jedi, installed via micropip alongside boto3.
      `jedi.Interpreter` completes against the live boto3 in the worker, so
      `boto3.`, a client, and its operation methods resolve against the real
      pinned library with no completion list to maintain. A `complete` worker
      message tagged with a sequence so a late reply is dropped; a
      keyboard-navigable listbox dropdown (up/down, Enter/Tab accept, Escape
      dismiss) with `role=listbox`/`aria-activedescendant`, positioned under
      the caret via CSSOM custom properties, which `style-src` does not govern
      so the policy is untouched. Browser-unverified.
- [x] Output pane showing stdout and the real traceback, separated from the
      wrapper message.
- [x] Run Python in a web worker; Stop terminates and rebuilds it, so a
      runaway loop is killed without the page and the filesystem reseeds.
- [ ] Browser-verify all of the above against the checklist below. This is the
      half the Rust side cannot reach.

## Markup

- [x] The two-column step layout, the workbench a sibling of the swap target
      rather than inside it, so Next leaves it mounted. The lesson region
      carries the `lab-lesson` id the swap will target in the next item.
- [x] The overview renders no workbench element at all, keyed off `as_call`
      rather than hidden by CSS.
- [x] A "Write the call" pane in the lesson column: the labelled snippets with
      the instruction to assemble them in the editor and Run. Server-rendered,
      so it teaches before the runtime loads and with script disabled. Tested
      that every snippet label reaches the page.
- [x] Without script: a `noscript` fallback shows the scaffold the editor
      would open on, and the snippets stay readable in the walkthrough, so the
      step teaches with the runtime absent. A real degradation, stated.
- [x] The lab's policy gains `script-src 'self'`, `wasm-unsafe-eval`,
      `connect-src 'self'`, and `worker-src 'self'`, on call steps only. The
      overview and landing keep a style-only policy as tight as the resume's,
      verified by a test asserting the overview carries no `script-src` and the
      call page does.
- [x] The swap: `assets/gen-ai/nav.js` intercepts Previous/Next, posts to a
      `/gen-ai/{id}/lesson` fragment endpoint, and replaces `#lab-lesson`'s
      interior while the workbench stays mounted. The lesson interior is a
      shared `lesson()` function so the fragment and the full page cannot
      diverge, verified by a test that the page contains the fragment
      verbatim.
- [x] The swap refuses to cross the CSP boundary. The overview is style-only
      and a call step has the widened policy; a fragment swap cannot change
      the document's CSP meta. The fragment carries `data-workbench`, and the
      script falls back to a full navigation when that differs from the
      current step, which is the only way to move the policy. In practice
      that is only overview-to-first-call and back. Browser-verify that the
      fallback actually navigates rather than swapping under the wrong policy.

## Infrastructure

- [~] Reversed: the CDN was a second vendor the CSP never allowed, and the
      first browser run hung on it. Pyodide and the wheels are now self-hosted
      on AWS. The small entry scripts (`workbench.js`, `nav.js`) stay embedded
      in the Lambda under `/gen-ai/assets/*`; the multi-megabyte runtime moves
      to an S3 asset bucket behind the same CloudFront behavior, served under
      `/gen-ai/assets/pyodide/`. Decision and runbook in `self-hosting.md`.
- [x] Worker `worker-src 'self' blob:` CSP fix; `PYODIDE_BASE` repointed at
      the self-hosted path; `boot()` installs wheels from same-origin URLs.
- [x] S3 asset bucket + CloudFront origin in `gen-ai-lab.ts`/`site-stack.ts`:
      private bucket with OAC, versioning, SSE, blocked public access,
      noncurrent-version lifecycle, immutable TTL. Behaviors ordered
      `/gen-ai/assets/pyodide/*` (S3) → `/gen-ai/assets/*` (Lambda) →
      `/gen-ai/*` (handler). `BucketDeployment` is conditional on the
      vendored directory existing, so a deploy before vendoring succeeds with
      an empty bucket rather than failing. `tsc` passes.
- [x] `vendor-pyodide.mjs`: one command fetches the pinned Pyodide core and
      runs the real micropip resolve, downloading every wheel and printing the
      `WHEELS` list. Binaries gitignored, lockfile kept, CI runs it before
      synth — git-as-truth via the pins and lockfile. Documented in
      `self-hosting.md`.
- [ ] Run `vendor-pyodide.mjs` and paste the emitted `WHEELS` list into
      `workbench.js`. Needs live network + Node; cannot be done from here.
- [x] Wire the vendor step into CI before `cdk synth`, in the deploy job
      where the Rust build is already paid for.
- [x] Content-hashed filenames, long immutable cache, verified locally.
- [x] A CloudFront behavior for `/gen-ai/assets/*` with a 365-day immutable
      cache, ordered ahead of `/gen-ai/*`. CloudFront matches in list order
      with no specificity sorting, so the assets pattern has to precede the
      catch-all or the runtime is refetched every visit. Confirmed against the
      synthesized template: `/gen-ai/assets/*` lists before `/gen-ai`.
- [ ] Confirm the resume's behaviors and policy are unchanged by diffing the
      synthesized template against a pre-lab synth. `tsc` passes and the
      changes are additive, but a byte diff is the real check and it is done
      with the deploy.

## Tests

The core-side tests landed with the model. The asset-side and browser tests
wait on the asset they describe.

- [x] The authored step parses, seeds its files, shows snippets, and names a
      response for every operation its snippets call.
- [x] The seed is a scaffold, not the answer: a test asserts the open file
      does not already call every intercepted operation.
- [x] Every snippet is not obviously broken: no tabs, every snippet labelled.
      A full Python parse is deferred to the browser run rather than a parser
      in the crate, which the no-dependency ban forbids anyway.
- [~] A `Reason` is well-formed and adds more than the exception name. The
      "matches nothing in the step" check is not written, because the
      exception a reason keys on is only known once the intercept runs in the
      browser; the crate cannot know which exceptions botocore will raise.
      Reframed from a build check to a browser-verify item below.
- [x] The overview renders no workbench.
- [x] No page carries a `style` attribute, so the policy still holds. Inherited
      from phase 1 and still passing.
- [ ] Snapshots of a call step and the landing. Still pending `insta`, same
      as phase 1.

## Verify in a browser

These cannot be asserted from Rust, so they are a checklist rather than a
suite. The phase is not done without them. The step-by-step — what to run,
what to type, and what counts as a pass for each — is in `browser-verify.md`
beside this file; the list here is the index into it.

- [x] The call assembled from the walkthrough's snippets into the empty editor
      runs and prints the intercepted response. Verified: prints the S3 URI,
      `end_turn`, and the cited German parental-leave answer.
- [ ] Dropping `modelId` raises `ParamValidationError: Missing required
      parameter "modelId"`. (The exercise was corrected: it is `modelId`, not
      `maxTokens`, that botocore requires. Verified against real botocore.)
- [ ] Dropping `inferenceConfig` does NOT fail — it succeeds, which is the
      lesson. An unbounded response is the cost mistake with no exception.
- [ ] Converse on the agent client raises `AttributeError` (no `converse`
      method), not a service error — it fails in Python before any request.
- [ ] Known boundary, not a check: a `global.` prefix does NOT error in the
      sandbox, because residency is validated server-side and the transport is
      intercepted. Documented in NOTES.md.
- [ ] A traceback's line number matches the gutter.
- [ ] `while True:` can be stopped without reloading the page.
- [x] Typing `boto3.` opens the dropdown with real boto3 attributes, and a
      client built in the buffer completes its operation methods. The
      completions match the pinned botocore, since jedi reads the live
      library. Verified working.
- [ ] The dropdown is keyboard-only operable: arrows move, Enter/Tab accept,
      Escape dismisses, and focus stays in the editor throughout.
- [ ] A completion requested while a cell is running does not freeze the
      editor, and a reply that lands after the caret moved is discarded rather
      than inserted.
- [ ] Next between two call steps swaps the lesson and the edited file is
      still open: the swap, not a navigation.
- [ ] Previous from the first call step back to the overview is a full
      navigation, not a swap, so the page lands under the style-only policy.
      Check the network tab shows a document load, not a `/lesson` fetch.
- [ ] A reload resets the filesystem to the step's scaffold, discarding the
      visitor's assembled call.
- [ ] The network tab shows the runtime fetched once, from the lab origin, and
      no request to AWS.

## Not in this phase

The other twenty steps of the policy assistant, the other scenarios, the
coverage test, AWS architecture icons, and the link from the resume.
