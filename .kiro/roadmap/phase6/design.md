# Phase 6 — Design

Status: domain layer built in `resume-core::fit`. Views and endpoint pending.

## Shape

```
POST /fit  (form post, text/plain body or urlencoded field)
  -> JobDescription::new          cap, normalize, fingerprint
  -> extract(taxonomy, text)      Vec<Requirement>
  -> for each requirement:
       search(corpus, text)       ranked CorpusEntry hits
       assess(hits)               Verdict + Citation list + rationale
  -> FitReport { requirements, assessments, provenance }
  -> report.validate(resume)      every citation resolves or the whole report
                                  is rejected
  -> render
```

One request, no persistence, no async. The fingerprint is retained as a cache
key for an ETag and for deduplicating identical submissions, not as a session
identifier.

## Why no state machine

Three of the five characteristics that justify orchestration are absent: there
is no step slow enough to exceed a request, no step that fails in a way worth
retrying separately, and no run worth inspecting after the fact. Fan-out over
requirements is a loop over a few dozen string comparisons.

Building a state machine here would be visible over-engineering, which is
precisely the thing the architecture is meant to demonstrate judgement about.

## Extraction without a model

A taxonomy in `content/skills-taxonomy.json`: canonical skill names with
aliases, so "JS", "JavaScript", and "ES6" collapse to one entry, and "LLM",
"GenAI", and "generative AI" to another.

Extraction then is:

1. Split the description into sections, detecting required and preferred
   headings by keyword.
2. Match taxonomy entries against each section.
3. `kind` comes from which section matched. Ambiguous means `niceToHave`.
4. `id` is the canonical skill slug, which makes requirements stable across
   submissions for the same posting.

Limitations, stated plainly because they are real: anything outside the
taxonomy is invisible, and a description phrasing a requirement unusually will
be missed. The mitigation is that the taxonomy is content in git, so a missed
requirement is a one-line content fix rather than a prompt tuning exercise.

That tradeoff is the honest version of the model comparison. A model would
generalize to unseen phrasing; the taxonomy will not. The taxonomy is free,
instant, deterministic, and explainable.

## Assessment without a model

`search` already ranks corpus entries by term overlap. Verdict from the hit
count relative to the requirement's term count, with thresholds tuned against
real job descriptions rather than guessed.

The rationale is templated: the matched terms plus the cited text. Arguably
better than generated prose, because it cannot editorialize or inflate.

## The seam

Extraction and assessment sit behind traits so a model implementation is an
addition rather than a rewrite. The prompts, the `RequirementSet` schema, and
`resolve_evidence_ids` already exist and are tested, so that path is designed
rather than merely imagined.

## UI

`/fit` is a page with a `<textarea>` and a submit button. It works with
JavaScript disabled, which is the baseline.

The modal is a `<dialog>` enhancement containing the same form. Drag-and-drop
accepts `text/plain` from the DataTransfer, which covers dragging a highlighted
selection from a job board and needs no parsing.

Visitor text is escaped on render, which maud does by default. The report shows
cited resume text, not the submitted text, so there is little surface for
reflected content in the first place.

## File uploads, and why not yet

Accepting PDF or DOCX means a parser dependency reachable from an
unauthenticated endpoint, with untrusted binary input. Zip bombs via DOCX,
malformed PDFs targeting allocators, and wildly inconsistent document structure
are all real.

If added: presigned S3 PUT so binaries never pass through the Lambda, a
separate extraction step with hard size and page limits, and explicit handling
for the "extraction produced nothing useful" case, which will be common.

That is a project, not a checkbox, and it does not change the orchestration
answer.

## Open

- Verdict thresholds. Need a handful of real job descriptions to tune against.
- Whether the taxonomy is hand-written or seeded from the skills already in
  `content/resume.json`. Seeding is tempting but would only ever match what the
  resume already claims, which defeats gap detection.

---

# 6A — Live provenance check

The state machine feature, added after the fit analysis became synchronous and
stopped justifying orchestration.

## Workflow

```
POST /provenance
  -> plan(resume, commit)        deterministic check list
  -> run_id(plan, commit)        cache key and execution name
  -> cached and fresh?           redirect to the existing run
  -> StartExecution

Plan                             write the queued projection
  -> Map over checks             MaxConcurrency bounded
       -> RunCheck               one outbound call per invocation
          Retry on transport errors, backoff with jitter
          ToleratedFailurePercentage: 100
  -> Summarize                   write the terminal projection
Catch -> RecordFailure           run-level failure only
```

`ToleratedFailurePercentage: 100` looks odd and is deliberate. A check that
cannot reach its target is a *result*, not a workflow failure. Every branch is
allowed to fail without failing the run, because "eleven of twelve passed, one
timed out" is the honest answer and the most likely one.

## Three outcomes, never two

The design hinges on separating these:

- `Passed` — reached it, it agreed.
- `Unreachable` — could not reach it. Says nothing about the claim. Corporate
  sites block datacenter IP ranges as a matter of policy, and a timeout from
  an employer's marketing site is not evidence about employment.
- `Mismatch` — reached it and it disagreed. The only outcome that means
  something is actually wrong.

Collapsing unreachable into failure would make the feature dishonest in the
direction that damages its author. Collapsing it into passed would make it
useless.

## What it cannot do

It checks links and public artifacts. It cannot verify employment, titles, or
dates, because there is nothing public to check them against.

This has to be stated in the UI copy, not just here. A feature called
"verified resume" that only checks whether URLs resolve would be the exact kind
of overclaiming the rest of this project avoids.

## Abuse

A visitor-triggered fan-out of outbound requests is an amplifier pointed at
third parties who did not opt in.

- `MAX_CHECKS` caps a run.
- `MIN_REFRESH_SECONDS` plus the run cache means repeated clicks reuse a run.
- Bounded `MaxConcurrency` on the `Map`, so a run is a trickle rather than a
  burst.
- Per-IP rate limiting at the edge on `POST /provenance`.

## Read model

Same rule as everywhere: the projection is written by the tasks and read by the
page. `DescribeExecution` cannot see mid-flight results and
`GetExecutionHistory` is not built for page-load traffic.

The projection lives in DynamoDB with a TTL. A lost run costs a visitor a
re-click, which is the correct amount of durability for this.

## Progress honesty

`CheckRun::progress` caps at 99 while the status is `Running`, even when every
planned result has arrived. A progress bar that sits at 100% while the page
still says "checking" reads as broken.

## Open

- Whether the GitHub check uses the unauthenticated API, which is rate limited
  per IP and shared across Lambda instances, or a token. Unauthenticated first,
  with the rate limit treated as `Unreachable`.
- Whether to show the check plan before running it. Probably yes: it sets
  expectations about scope and makes the "cannot verify employment" point
  concrete rather than a disclaimer nobody reads.
