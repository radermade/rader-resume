# Phase 6 — Tasks

Status: not started

## Decide first

- [ ] Bedrock model, justified by an eval run
- [ ] Retrieval: keyword matching with synonyms, or embeddings and vector
      search (see design)
- [ ] Numbers: input size cap, per-IP rate limit, daily ceiling, monthly budget
- [ ] Is the tailored variant download-only or shareable

## Guardrails, before anything is public

- [ ] Input size cap enforced in the handler before any model call
- [ ] Per-IP rate limiting at the edge
- [ ] Daily execution ceiling enforced in the handler
- [ ] Bedrock spend budget and alarm
- [ ] Confirm raw input and prompts are absent from logs

## Types and prompts

- [ ] Requirement, Evidence, Citation, FitAssessment types in `resume-core`
- [ ] Citations hold role slugs and highlight indices, validated against the
      document so a fabricated citation fails deserialization
- [ ] Prompts as versioned files in the repo
- [ ] Eval set: real job descriptions with hand-labelled expected output
- [ ] Eval run wired into CI

## Task binaries

- [ ] `tasks/fit-ingest`: normalize, hash, write to S3
- [ ] `tasks/fit-extract`: schema-constrained extraction
- [ ] `tasks/fit-match`: one requirement per invocation
- [ ] `tasks/fit-score`: assemble, rank, gap list
- [ ] `tasks/fit-render`: tailored HTML and PDF

## Workflow

- [ ] ASL with the cache `Choice`, `Map`, and deliberate tolerated failure
- [ ] Throttling retries distinct from the non-retryable validation path
- [ ] Execution naming from the input hash
- [ ] Failure path writing a readable diagnostic

## Handler and UI

- [ ] `POST /fit` with size cap, rate check, ceiling check
- [ ] `GET /fit/{id}` reading the projection, zero Step Functions calls
- [ ] Progress view, htmx-enhanced, working without JavaScript
- [ ] Result view with citations linked to resume entries
- [ ] Escape all model-generated text

## Infrastructure

- [ ] DynamoDB sessions table: session ID partition, TTL
- [ ] S3 prefix for submitted text and analysis artifacts, with lifecycle
- [ ] Per-task IAM, Bedrock invoke scoped to the chosen model
- [ ] CloudFront behavior for `/fit/*`, no-store
- [ ] Logging, X-Ray, failure alarm

## Verify

- [ ] Submit the same description twice, confirm the cache hit
- [ ] Submit a description containing injected instructions, confirm structural
      defense holds
- [ ] Script a loop, confirm the ceiling stops it
- [ ] Confirm a fabricated citation cannot render
