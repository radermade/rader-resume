---
inclusion: always
---

# AI Integration Standards

Scope: LLM calls from Lambda, currently Bedrock, currently the Phase 6 fit
analysis. These rules exist because model output is untrusted input that costs
money.

## Output is a contract

- Every model call that produces structured data deserializes into a named
  `resume-core` type or the task fails. No free-form parsing, no regex over
  prose, no "usually it returns JSON."
- Constrain the output at the API level with a JSON schema where the model
  supports it, then validate again on deserialization. The schema in the
  request and the Rust type are generated from or checked against the same
  definition, the same way the OpenAPI-to-TypeScript pipeline works.
- A validation failure is not retryable with the same input. Retrying a
  malformed response usually produces another malformed response. Route it to
  the failure path with the raw output attached as a diagnostic.
- Model output never drives control flow directly. A `Choice` state branches on
  a validated field, never on a model's free text.

## Input is hostile

- Visitor-supplied text reaching a prompt is untrusted. A pasted job
  description can contain instructions aimed at your extractor.
- Defend structurally rather than by asking the model nicely. Separate
  instructions from data in the prompt, cap input size, and validate that the
  output conforms to the expected shape regardless of what the input asked for.
- Treat an output that fails validation as a possible injection attempt, not
  only a model error. Log that it happened.
- Never let model output reach a tool call, an SDK call, or a rendered page
  without validation. Rendered model text is escaped like any user content.

## Prompts are content

- Prompts live in git, versioned, reviewed in pull requests like any other
  content. Not inline string literals scattered across handlers.
- Every result records the model ID and the prompt version that produced it, so
  an output can be explained months later.
- Low temperature for extraction and classification. Determinism matters more
  than variety when the output feeds a schema.
- A prompt change without an eval run is a guess. Keep a small labelled set of
  real job descriptions with expected extraction output in the repo, and run it
  in CI. Prompt regressions are silent otherwise.

## Cost and abuse

Any unauthenticated endpoint that triggers a model call is a cost
amplification target. All of the following exist before the feature is public:

- Hard input size cap, rejected at the handler before any model call.
- Per-IP rate limiting at the edge.
- Daily execution ceiling enforced in the handler, not only by alarm. An alarm
  tells you after the money is gone.
- Monthly Bedrock spend budget with an alarm well below the pain threshold.
- Cache by normalized input hash, so resubmitting the same job description
  costs nothing. This helps against repetition, not against randomized input,
  which is what the ceiling is for.

## Privacy and logging

- Never log raw visitor input or full prompt bodies. Log the input hash, token
  counts, model ID, prompt version, latency, and outcome.
- Visitor-pasted text may contain personal data. TTL it with the analysis
  record and do not retain it beyond the result's usefulness.
- Model-generated claims about experience are cited back to evidence in the
  resume corpus, or they are not displayed. An uncited assertion about a career
  is a hallucination risk with a real cost.

## Orchestration

- One model call per task state. A task making three chained calls is three
  states, and collapsing them hides which one failed and costs the retry
  granularity Step Functions exists to give you.
- Throttling is the dominant failure mode. Explicit `Retry` with backoff and
  jitter on throttling errors, distinct from the non-retryable validation path.
- Model latency is measured in seconds. Design for it in the workflow rather
  than holding an HTTP request open.
