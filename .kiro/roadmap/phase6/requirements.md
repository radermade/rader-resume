# Phase 6 — Job Description Fit Analysis

Status: not started. The flagship feature.

Goal: a recruiter pastes the job description they are hiring for and gets a
structured mapping of this experience against their actual requirements, with
cited evidence, a gap list, and a tailored resume variant as a PDF.

## Functional

- R6.1 `POST /fit` accepts job description text and starts an analysis.
- R6.2 `GET /fit/{id}` returns current progress, then the result.
- R6.3 Requirements are extracted from the job description as structured data.
- R6.4 Each requirement is matched against the resume corpus with cited
  evidence and a rationale.
- R6.5 Output includes a ranked fit assessment and an explicit gap list.
- R6.6 A tailored resume variant is generated as HTML and PDF.
- R6.7 Resubmitting the same job description returns the cached analysis.

## Constraints

- R6.8 Job description text goes to S3 on ingest. Only the key travels in the
  workflow payload.
- R6.9 The normalized input hash becomes the execution name, so duplicate
  submissions collide instead of running twice.
- R6.10 `GET /fit/{id}` reads a DynamoDB projection and makes zero Step
  Functions calls.
- R6.11 Model output must deserialize into a `resume-core` type or the task
  fails. Validation failures are not retried.
- R6.12 One model call per task state.
- R6.13 No `waitForTaskToken`. This is an async job with progress polling.
- R6.14 Session items carry a TTL. Abandoned analyses evaporate.
- R6.15 Gap and fit claims are cited to evidence in the corpus or not shown.
- R6.16 Raw input and full prompts are never logged.
- R6.17 Works without JavaScript. htmx enhances the progress view.

## Abuse and cost, all required before public launch

- R6.18 Hard input size cap, enforced before any model call.
- R6.19 Per-IP rate limiting at the edge.
- R6.20 Daily execution ceiling enforced in the handler, not only by alarm.
- R6.21 Monthly Bedrock spend budget with an alarm below the pain threshold.

## Exit criteria

A pasted job description produces a cited fit analysis and a downloadable
tailored PDF, repeated submissions hit the cache, and a scripted loop against
the endpoint cannot run up a bill.
