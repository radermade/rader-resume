# Phase 6 — Tasks

Two features: 6A the provenance check (state machine), 6B the fit analysis
(synchronous).

## Done

Shared:

- [x] `JobDescription` as the only entry point for visitor text, with cap and
      normalization
- [x] Fingerprint-as-execution-name pattern, reused by both features

6B fit analysis:

- [x] `Requirement`, `Citation`, `Assessment`, `FitReport`, `Provenance` types
- [x] Structural citations that resolve against the live document
- [x] `FitReport::validate` rejects the whole report if any claim is uncited
- [x] Corpus flattening and keyword search
- [x] Weighted scoring, must-haves counted double
- [x] Retrieval decision: keyword matching, not embeddings
- [x] Removed the async scaffolding. `Analysis` and `AnalysisStatus` existed
      to absorb model latency and are gone

6A provenance check:

- [x] `resume-core::provenance`: `Check`, `CheckKind`, `Outcome`,
      `CheckResult`, `CheckRun`, `RunStatus`
- [x] `plan()` derives checks deterministically from content and skips
      bracketed placeholder URLs, because an unfilled blank is not a dead link
- [x] Three-way outcome: passed, unreachable, mismatched, with `is_problem`
      true only for mismatch
- [x] GitHub URL parsing into owner and repo, falling back to a plain link
      check
- [x] `run_id()` over the plan plus commit, so a publish gets a fresh run
- [x] `MAX_CHECKS` and `MIN_REFRESH_SECONDS` as documented limits
- [x] Progress caps at 99 while running, so the bar never lies

## Done — 6A, local

The feature works end to end locally with real outbound HTTP, before any AWS
resource exists. That ordering was deliberate: the orchestration is the last
thing added, not the first.

- [x] `crates/check-runner`: executes one check, shared later by the task
      Lambda. Separate crate so neither caller depends on the other, and so
      `resume-core` keeps no HTTP dependency
- [x] Status classification: 404 and 410 are `Mismatch`, 401/403/429 and 5xx
      are `Unreachable`, transport failures are `Unreachable`. Tested, because
      this is where the feature is honest or it isn't
- [x] Artifacts treated as `Mismatch` on failure, since this site serves them
- [x] `/api/build` endpoint, `no-store`, for the live-commit comparison
- [x] Views: plan, progress, results, with the scope disclaimer on both
- [x] `meta refresh` polling, dropped once terminal, so progress works with
      JavaScript disabled
- [x] `MemoryRuns::claim`, so a second click reuses the run rather than
      starting a duplicate fan-out
- [x] Run page and `/api/build` are `no-store`
- [x] Sequential local runner, matching the bounded concurrency the deployed
      version will use

## Done — 6A, deployed

- [x] Gate driven by whether an execution path actually works, rather than by
      environment. State machine anywhere, or in-process locally. Off in
      Lambda without the state machine, because a `tokio::spawn` run does not
      survive the environment freezing after a response
- [x] DynamoDB runs table, partitioned on run id, TTL, on-demand billing
- [x] `run-store` crate: `Runs` enum over memory and DynamoDB, shared by the
      handler that reads and the task that writes
- [x] Item shape reworked from a JSON blob to real attributes with
      `list_append`, because parallel branches doing read-modify-write would
      silently lose each other's results
- [x] `crates/provenance-task`: begin, one check per invocation, finish, and a
      failure recorder, reusing `check-runner` so the logic has one copy
- [x] Standard workflow: `Begin`, bounded `Map`, `Finish`, with a `Catch` to
      `RecordFailure` carrying a fixed visitor-safe diagnostic
- [x] `maxConcurrency: 4`, because this fans out at third parties who did not
      opt in
- [x] Handler starts an execution named after the run id, so a duplicate that
      slips past the conditional write collides at the service instead
- [x] CloudFront behaviors for `/provenance`, `/provenance/*`, `/api/build`,
      all methods, caching disabled

## Remaining

- [ ] Deploy and verify: trigger a run, watch the projection fill, confirm an
      unreachable host reports honestly rather than as a failed claim.
      Still the gating item. Everything below it is written but has never run
      against real AWS
- Declined: per-IP rate limiting on `POST /provenance` at the edge. A WAF web
  ACL carries about a five dollar monthly standing charge, which is the single
  largest line item this project would have, to protect an endpoint already
  bounded two ways: `MAX_CHECKS` caps the fan-out per run, and
  `MIN_REFRESH_SECONDS` plus the conditional claim means repeated clicks reuse
  the existing run instead of starting new ones. The application-level bounds
  are the real control; the WAF would be a paid second opinion. Revisit only
  if logs show someone actually trying.
- [x] Alarm on failed executions for both workflows, plus handler `Errors` and
      `Throttles`, all on one SNS topic. The address comes from
      `--context alarmEmail=` rather than a literal, so no personal contact
      detail lands in the repo. Without it the alarms deploy unrouted and the
      stack output says so
- [x] Resolved by deletion: the `GitHubRepo` check kind is gone, and with it
      the token question. Repository URLs are now plain `ExternalLink` checks.

      The check called `api.github.com/repos/{owner}/{repo}` unauthenticated
      to satisfy R6.5's last-push date. That is 60 requests an hour per
      source IP, and Lambda egresses from shared AWS addresses, so the budget
      was shared with strangers and the ceiling was unpredictable rather than
      merely low. The honest `Unreachable` classification meant it never
      lied, but "could not reach" on a repository that plainly exists is a
      confusing answer to a question the visitor did not ask.

      A token would have fixed the limit and introduced a long-lived
      credential to rotate, against a project that uses OIDC everywhere else
      specifically to avoid one. Neither option was worth it for a date
      nobody requested. R6.5 is struck.

      The underlying want, letting a technical reader see the stack, is a
      different feature and does not belong in a verification workflow. See
      6C below.
- [x] Retry with backoff and jitter on the `Map` branches, slower than the fit
      workflow's because retries here become outbound requests at third
      parties
- [x] Retry inside `check-runner` before it concludes `Unreachable`. Split out
      from the item above, which conflated two different things: a target that
      times out does not fail the task, so no Step Functions retry can ever
      fire for it. Fixing a transient blip being reported as unreachable means
      retrying before the outcome is decided, which is a change in the crate,
      not in the workflow.

      Two attempts, 400 ms apart, and narrow about what qualifies. Refused
      connections and 5xx are retried. A 404 or 410 is a definite answer, a
      401 or 403 is a wall a retry from the same address cannot pass, and a
      429 is an explicit instruction to slow down that retrying would
      disobey.

      Timeouts are excluded, which is the non-obvious part. A host that spent
      the full eight second budget not answering is not having a blip, and
      retrying it is what would blow the task's twenty second ceiling: two
      timeouts plus the delay is 16.4 seconds before a result is even
      written. There is a test asserting the worst case still fits.

## Next — 6B

Revised to an Express workflow with a four-branch `Parallel`. See
`design-6b.md`. Deterministic throughout.

Domain, pure:

- [x] `Dimension` on `Requirement`: tech, skills, experience, seniority.
      Without it the four branches would run identical logic on identical
      input, which is the version that would be theatre
- [x] `Tier` band over the score, so a bare number does not imply precision
- [x] `content/skills-taxonomy.json`: canonical entries with aliases and a
      dimension each
- [x] `extract()`: section detection for required versus preferred, taxonomy
      matching, dimension assignment
- [x] Dropped-requirement count. Was a hardcoded zero, which made R6.20's
      coverage disclosure a lie: the field existed, was rendered nowhere, and
      always said nothing was missed. Now counts clauses in the required
      blocks that no taxonomy entry explains, and only the required blocks,
      because counting boilerplate would make the number large enough to
      ignore. Carried through `ExtractOutput` and `AggregateInput` because the
      description is dropped after extraction and the count cannot be
      recovered from the requirements alone
- [x] Four matchers, one per dimension, each over its own corpus slice
- [x] `Experience` matcher using tenure arithmetic from `derived.rs` for
      "5+ years" style requirements. The one branch with non-trivial logic
- [x] `aggregate()`: merge branch outputs, score, tier, gaps, and three to
      five emphasis points deduplicated by role
- [ ] Calibrate tier boundaries against real job descriptions kept as
      fixtures. Needs real postings; the current boundaries are a guess

Deployed:

- [x] `crates/fit-task`: one binary dispatching on a tagged payload, serving
      all six states. They are pure compute with identical resource needs, so
      separate artifacts would be copies. Rust standards amended with the
      reasoning rather than quietly broken
- [x] Task payload types in `resume-core::tasks`, deliberately tolerant of
      unknown fields, with a test asserting an added field does not break a
      task
- [x] `FitWorkflow` construct: Express state machine, `Extract` into a
      four-branch `Parallel` into `Aggregate`
- [x] Execution logging enabled, since Express keeps no history of its own and
      the per-branch timing is one of the reasons for using a workflow
- [x] Execution data logged in dev, withheld in prod, because visitor text
      reaches the Extract input
- [x] `resultSelector` drops the description after extraction, so visitor text
      is not carried through the remaining states
- [x] `grantStartSyncExecution` only. The handler has no reason to start a
      detached execution
- [x] `POST /fit` calling `StartSyncExecution` when `FIT_STATE_MACHINE_ARN` is
      set, falling back to inline otherwise. Local development needs no
      credentials, and a workflow failure degrades rather than breaks
- [x] Alarm on the fit state machine's failed executions. Not optional: the
      inline fallback means a broken workflow is invisible without it
- [x] `Catch` producing a partial report rather than no report. Per branch, to
      a `Pass` returning that dimension with no assessments. `aggregate`
      derives the incomplete dimensions by comparing requirements against
      assessments rather than trusting a flag, so it is right whether a branch
      failed, timed out, or was never invoked, and the report names what is
      missing instead of quietly omitting it
- [x] Explicit `Retry` with full jitter on every task state, and
      `retryOnServiceExceptions: false` to suppress CDK's default. Left on,
      the default renders first and shadows the overlapping errors, so the
      explicit policy would have contributed nothing but throttling and the
      jitter would have been decorative. Confirmed against the rendered ASL,
      not assumed
- [x] Fingerprint cache in the handler, since Express does not deduplicate by
      execution name the way a Standard execution name collision does. A
      repeat submission or a reload after a form post now skips the workflow.

      Per-instance and deliberately not promoted to DynamoDB: a miss
      recomputes deterministic work measured in microseconds, so a cold
      instance is invisible, and a visitor's pasted text is not worth
      persisting for a performance gain nobody would notice. Safe without a
      TTL because the analysis is pure over content embedded at build time,
      so nothing can change within an instance's lifetime that would make an
      entry wrong.

      Capped at 32 entries and cleared rather than evicted when full. An
      unauthenticated endpoint should not let a stream of distinct postings
      grow a warm instance's memory, and tracking recency to protect
      microseconds of arithmetic would cost more than the thing it protects.
      Stored only after validation, so a rejected report cannot be served
      from cache later. Responses stay `no-store` regardless.
- [ ] Migrate the workflow to JSONata. Built with JSONPath to avoid stacking
      another CDK API version guess onto an unverified deploy, which is a
      reason to defer rather than a reason to prefer it
- [x] `/fit` page with textarea and form post, the no-JavaScript baseline
- [x] `<dialog>` modal on the home page, with the trigger as a real link to
      `/fit` that script only intercepts where `showModal` exists
- [x] The report renders into the modal rather than navigating away. The
      point of the feature is a recruiter or engineer reading the resume, and
      sending them to a different page to see the answer threw that away.

      `POST /fit/fragment` returns the report markup alone. A separate route
      rather than content negotiation on `/fit`, because fragment endpoints
      return fragments and page endpoints return documents, and deciding
      which deep inside one handler is how that distinction rots.

      Both surfaces call one `fit_report_body`, so the modal and the page
      cannot report different analyses. A test asserts every requirement
      appears on both or neither.

      Heading depth is a parameter: the page owns its document and titles the
      report `h1`, the fragment lands inside a page whose `h1` is the profile
      and so starts at `h2` with everything below shifting down. Without that
      the modal would wreck the page's heading outline.

      The no-JavaScript path is unchanged. The form in the dialog still posts
      to `/fit` on its own, and the script's failure branch calls
      `form.submit()`, so an unexpected error degrades to a full page rather
      than a dead button.

      `connect-src 'self'` added to the policy for the one fetch. Same origin
      only, so the pasted description cannot be sent anywhere else.
- [x] CloudFront behavior for `/fit/fragment`. `/fit` is an exact pattern and
      does not cover it, so without its own behavior the POST falls to the
      read-only default and CloudFront rejects it before the origin sees it.
      The modal would have worked locally and failed everywhere else, which
      is the kind of bug that only a deploy finds.
- [x] Dropped `text/plain` into the textarea. Files stay out of scope
- [x] Shared form fields between page and modal, so they cannot drift
- [x] UI copy: coverage limits, and that the score measures overlap with
      documented experience rather than suitability
- [x] Content Security Policy. `default-src 'none'`, opened only as far as
      something needs: sha256 hashes for the two inline blocks, `form-action
      'self'` for the fit post, `img-src 'self' data:` for the portrait Phase
      3 adds. No `unsafe-inline`, no `unsafe-eval`, no wildcard.
      `connect-src` stays closed; htmx for the stepped flow would need
      `'self'` there, as a deliberate change rather than a pre-opened hole.

      Hashes, not a nonce. A nonce must be unique per response, which makes
      every page uncacheable and forecloses the pre-rendered static artifacts
      that are the whole target for reads. Hashes are deterministic, so the
      same policy is correct in a handler response and in a file on S3.

      The script moved from a literal inside the template to a constant, so
      the policy hashes exactly the bytes the document inlines. Two copies
      would drift, and a drifted script hash means the modal silently stops
      opening, which is a bug that survives a long time.

      Delivered as a `meta` element, which is a real tradeoff:
      `frame-ancestors`, `report-uri`, and `sandbox` are ignored there and
      must come from the edge. In exchange the policy travels with the
      document, so a pre-rendered artifact carries its own protection instead
      of depending on a CloudFront response-headers policy staying in sync
      with it. Clickjacking protection belongs at the edge next to HSTS
      anyway.
- [x] Regression test that no rendered page uses a `style=` attribute or an
      inline event handler. A block hash covers a `<style>` element, not an
      attribute, so the first ornament that needs one would arrive with a
      proposal to add `'unsafe-hashes'` and quietly undo the policy. Keeping
      them out is the condition that lets the policy stay strict, so it is
      enforced rather than remembered.

## Next — 6C, the stack page

Not a provenance check. Disclosure, not verification: no outbound requests,
nothing that can be unreachable, no run to poll, no state machine. Naming it
a check would saddle a static page with a fan-out workflow.

The want: let a technical reader see what the project is built from without
being sent to GitHub.

- [x] Route `/stack`, server-rendered from files embedded with `include_str!`
      the way `resume.json` and the taxonomy already are. Embedding at build
      time means the page shows the manifests from the commit that built it,
      which is the same git-as-truth property the footer SHA has, and it
      makes the page a pre-renderable static artifact under Phase 4.
      Rendered once at cold start and served with an ETag, like the home page
- [x] `Cargo.toml` verbatim. The dependency rationale comments are the
      interesting part, not the version numbers
- [x] `infra/package.json` verbatim
- [x] Linked from the footer. Unobtrusive, next to the commit SHA: a
      recruiter has no use for it, and the engineer who wonders what this
      runs on will look there
- Rejected: `cdk.json` verbatim. Its context carries `hostedZoneId`, a live
  AWS resource identifier for the account. Not a credential, but no upside to
  publishing, and a hand-maintained redaction list fails open the next time a
  context key is added. An allowlist would fail closed, but the file's
  genuinely interesting content is the architecture rather than the config.
- [x] An architecture diagram instead, which is what the reader wanted from
      `cdk.json` anyway.

      Semantic HTML and CSS, not an image and not Mermaid. Mermaid needs
      JavaScript, which breaks "no JavaScript required to read any content"
      and the 100 KB first-view budget. HTML nodes are readable by a screen
      reader without a hand-written `alt` paragraph, themeable from the
      existing custom properties, and free.

      Ordered bands rather than drawn edges. Routing arrows in CSS means
      hand-positioned coordinates, which is precisely what the old site did
      and what this replaces; bands convey the same dataflow and lay out with
      grid.

      This is the hexagon's job. One element, one rule, sized as a bullet
      rather than an ornament, `aria-hidden` because the label beside it
      carries the meaning.
- [x] Nodes as data in `content/architecture.json`, not markup. Same reason
      the bracket motif was stripped from `resume.json`: the record is data,
      the personality is in the renderer. `deny_unknown_fields` and an
      explicit `schemaVersion`, because this is authored content where an
      unknown field is a typo rather than an orchestration layer adding
      something.
- [x] CI check that every AWS service named in the diagram appears in the
      synthesized CloudFormation template:
      `infra/bin/check-architecture.ts`.

      A hand-maintained diagram drifts, and drift on a page whose whole
      premise is honesty is worse than having no diagram. Asserting against
      the cloud assembly makes it self-verifying, which gets the provenance
      property without any of the workflow.

      It compares against an existing assembly rather than synthesizing its
      own, because synthesizing builds every Rust binary. The deploy job now
      synthesizes once, runs the check, and deploys with `--app cdk.out` from
      that same assembly, so the build is paid for once. That also means a
      wrong diagram blocks a deploy rather than a pull request; the
      document's own invariants are unit-tested, so a PR still catches a
      malformed one.

      Deliberately one-directional: it proves the diagram claims nothing the
      stacks lack, not that the diagram is complete. Failing on undrawn
      resources was considered and rejected, because the stacks are full of
      IAM policies, log groups and Lambda permissions that are plumbing
      rather than architecture, and forcing all of them onto a diagram
      produces a worse diagram. The page says "checked rather than
      exhaustive" instead of implying more.
- [x] Guard test asserting no published manifest carries a hosted zone id or
      anything account-shaped, so the reason `cdk.json` was excluded is
      enforced rather than remembered

## Verify

- [ ] Trigger twice inside the refresh window, confirm the run is reused
- [ ] Point a check at a host that blackholes, confirm `Unreachable` and that
      the run still completes
- [ ] Confirm progress never shows 100 while the status is running
- [ ] Confirm the results page never calls `DescribeExecution`
