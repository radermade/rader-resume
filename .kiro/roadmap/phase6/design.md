# Phase 6 — Design

Status: shape settled. Model choice and thresholds open.

Rules for model calls live in `.kiro/steering/ai-integration-standards.md`.

## Workflow

```
Ingest              normalize, hash, write text to S3, return the key
  -> CheckCache     Choice on the hash: existing analysis? return it
  -> ExtractRequirements   Bedrock, JSON-schema-constrained output
                           Retry on throttling with backoff and jitter
                           validation failure -> RecordFailure, no retry
  -> Map over requirements
       -> MatchRequirement   retrieve evidence, produce cited rationale
       ToleratedFailurePercentage: 10
  -> ScoreAndAssemble
  -> RenderVariant  tailored HTML + PDF under the analysis prefix
  -> Finalize       projection status complete
Catch on every task -> RecordFailure with a readable diagnostic
```

## Why this feature and not a career walkthrough

A linear tour of a career is a wizard with no decisions in it. Step Functions
would be visibly over-engineered, and a reviewer who knows the service spots a
misapplied state machine faster than they notice one was used at all. That
would work against the exact claim the project exists to support.

This feature instead has four of the five characteristics that justify
orchestration: multiple steps with distinct failure modes, work too slow for a
request, fan-out over independent items, and retries that matter.

It is also useful to the person it is aimed at, and it produces the per-role
variant generation that is the single feature most likely to actually land
interviews.

## Read model

Each task updates a DynamoDB item keyed by analysis ID with its current step
and status. `GET /fit/{id}` serves from that item.

Reads never touch Step Functions. `DescribeExecution` returns status and input
with output only after completion, so mid-flight data is invisible;
`GetExecutionHistory` is paginated, rate-limited, and capped at 25k events.
Neither is built for per-page-load traffic.

## Caching by input hash

The normalized hash is both the cache key and the execution name. Identical
submissions collide at the service level rather than needing application
locking. This is the same idempotency approach the publish pipeline uses with
commit SHAs.

It defends against repetition, not against randomized input, which is what the
daily ceiling is for.

## Matching

`MatchRequirement` retrieves candidate evidence from the resume corpus, then
asks the model to produce a rationale citing specific entries. The citation is
structural: the output type holds role slugs and highlight indices, validated
against the actual document, so a fabricated citation fails deserialization.

Open question: whether retrieval is keyword matching over a small corpus or
embeddings and vector search. The corpus is one resume. Keyword matching with
synonym expansion is probably sufficient and far cheaper; embeddings would
demonstrate more current technique. Decide on merit, not on resume keywords.

## Prompt injection

A pasted job description is untrusted and can contain instructions aimed at the
extractor. Defense is structural: instructions separated from data, input
capped, and output validated against the expected shape regardless of what the
input asked for. An output that fails validation is logged as a possible
injection attempt, not only as a model error.

Rendered model text is escaped like any user content.

## Open questions

- Which Bedrock model, and the eval set to justify it. Needs a small corpus of
  real job descriptions with hand-labelled expected extraction.
- Specific numbers for input cap, rate limit, daily ceiling, monthly budget.
- Whether the tailored variant is offered as a download only, or also as a
  shareable link, which would turn an ephemeral analysis into a published
  artifact with different retention implications.
