# 6B — Job Description Fit Analysis

Status: types and corpus search built. Extraction, branches, and aggregation
outstanding.

Deterministic throughout. No foundation model.

## Workflow

Express, invoked with `StartSyncExecution`, so the handler gets the result
inside the request.

```
POST /fit
  -> JobDescription::new            cap, normalize, fingerprint
  -> StartSyncExecution

Extract                             taxonomy + section detection
                                    each requirement classified by dimension
  -> Parallel
       MatchTech                    languages, frameworks, tools, platforms
       MatchSkills                  practices: testing, accessibility, review
       MatchExperience              domain, industry, scale, regulated context
       MatchSeniority               leadership, mentoring, ownership, impact
  -> Aggregate                      merge, score, tier, gaps, emphasis
Catch -> RecordFailure

  <- FitReport returned to the handler, rendered, done
```

## Why Express and sync

Standard plus polling would need a projection, a store, a poll endpoint, and a
TTL, all to absorb latency that does not exist. Express with
`StartSyncExecution` keeps the orchestration and deletes that entire layer.

Consequences accepted:

- Express does not deduplicate by execution name, so the fingerprint cache, if
  wanted, lives in the handler rather than being free from the service.
- Express has no execution history by default; logging must be enabled
  explicitly or per-branch timing is invisible, which is one of the reasons for
  using a workflow here at all.
- Five minute ceiling, irrelevant at this scale.

## Why orchestration at all

Stated honestly: not for throughput. Four deterministic matchers over a few
dozen strings are microseconds of work, and state transitions cost more than
the work they coordinate.

The real reasons:

- Each branch is a different matcher with different logic, a different corpus
  slice, and different scoring weights. The `Parallel` state is the shape of
  the problem, not decoration over a loop.
- Per-branch failure isolation. A broken seniority matcher yields a partial
  report rather than no report.
- Per-branch timing in the execution log, which is what makes the tradeoff
  inspectable rather than asserted.
- A seam. If any branch later needs I/O, a taxonomy fetched from S3 or a model
  call, the structure already supports it without a rewrite.

This is a deliberate choice to spend latency on legibility. It is defensible
precisely because the same judgement rejected orchestration for the provenance
read path and for the career walkthrough.

## Dimensions

Without this the four branches would run the same logic on the same input,
which would make the `Parallel` theatre.

| Dimension | Matches against | Example requirement |
| --- | --- | --- |
| `Tech` | skills groups, role highlights mentioning tools | "TypeScript", "AWS Lambda" |
| `Skills` | practices in skills groups and highlights | "testing", "accessibility", "code review" |
| `Experience` | role employers, titles, domain language, tenure | "regulated industry", "5+ years frontend" |
| `Seniority` | highlights describing ownership, mentoring, scope | "mentor junior engineers", "own architecture" |

Extraction assigns the dimension from the taxonomy entry that matched. An
unmatched requirement is dropped rather than guessed at, and the count of
dropped requirements is reported so the result is honest about its own
coverage.

## Output

- `score`: 0 to 100, must-haves weighted double, already implemented.
- `tier`: a band over the score, because a number alone invites false
  precision. Strong, Solid, Partial, Weak.
- Matched requirements, each with the resume text that supports it.
- Gaps, stated plainly. A report that matches everything is useless.
- Three to five emphasis points: the role highlights that satisfied the most
  must-have requirements, deduplicated by role, phrased as what to lead with.

Emphasis points are the genuinely useful output. They answer "what should I
say first in this interview", which is a question no job board answers.

## Honest limits, to state in the UI

- Requirements outside the taxonomy are invisible. The mitigation is that the
  taxonomy is content in git, so a miss is a one-line fix.
- Keyword matching does not understand phrasing it has not seen.
- The score reflects overlap with documented experience, not suitability. A
  low score may mean a bad match or may mean the resume does not yet say
  something true.

## Input

Paste, or drag a highlighted selection. `text/plain` from the DataTransfer, no
file parsing. `/fit` works with a textarea and a form post; the modal and
drag-and-drop are enhancements.

File upload stays out of scope: PDF and DOCX parsing from an unauthenticated
endpoint is untrusted binary input, and it does not change the orchestration
answer.

## Open

- Tier boundaries. Need real job descriptions to calibrate against, kept as
  fixtures.
- Whether `Experience` matching should use tenure arithmetic from
  `derived.rs` for "5+ years" style requirements. Probably yes, and it is the
  one branch with non-trivial logic.
