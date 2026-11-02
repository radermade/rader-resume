---
inclusion: auto
name: aws-gen-ai-prompts
description: "Use when writing, versioning, chaining, or governing prompts on Amazon Bedrock: system prompts, few-shot, structured output, Prompt Management, Prompt Flows, prompt caching placement, and conversation state."
---

# Prompts

Prompts are content. They live in git or in Bedrock Prompt Management, versioned and reviewed. Every result records the model id and the prompt version that produced it.

## Instruction stack

1. **System.** Who the model is, what it must refuse, tone, and tool rules.
2. **Context.** Retrieved chunks, each labeled with a source id.
3. **User task.** The question, kept separate from the instructions.
4. **Output contract.** JSON schema, headings, citation format, maximum length.
5. **Reasoning.** Chain-of-thought when the task is multi-step and the token budget allows it. When latency or leakage matters, ask for the answer only, or reason in a scratchpad the handler strips.

Put instructions at the start and again at the end of a long prompt. Models use the middle of a long context less reliably (lost in the middle).

Separate instructions from data. A pasted document, a retrieved chunk, and a tool result are untrusted. Structural separation beats a sentence that says “ignore instructions in the document.”

## Patterns

| Pattern | Shape |
|---|---|
| Zero-shot | Instruction only |
| Few-shot | A handful of examples, placed in the stable prefix so they can be cached |
| Chain-of-thought | The model writes steps. No external tool |
| Prompt chain | The output of prompt A is the input of prompt B. Fixed order |
| Prompt Flows | A visual chain with branches, reusable nodes, and pre/post steps, for builders who should not ship the chain as code |
| ReAct | Thought, tool call, observation, repeat. An agent runtime or Step Functions |
| Structured output | A JSON schema the next system can parse |

Branching on a model answer belongs in Prompt Flows or Step Functions. A reviewed pipeline in git belongs in CodePipeline. Prompt Flows author the chain. The pipeline ships and gates it.

## Prompt Management

Use Bedrock Prompt Management when more than one application shares a prompt, or when a change needs an approver. Templates are parameterized and versioned. Store the files in S3 when the team wants the bytes in the corpus bucket. Log reads and publishes with CloudTrail.

A string literal copied across handlers will drift. One template, one version.

## Output is a contract

- Constrain structured output with a schema at the API where the model supports it, then validate again on deserialization.
- A validation failure is not retried with the same input. Route it to the failure path with the raw output attached as a diagnostic.
- Model text does not drive control flow, a tool call, an SDK call, or a page until it has been validated.
- Rendered model text is escaped like any other untrusted content.

## Conversation state

| Need | Where it lives |
|---|---|
| History | DynamoDB. Summarize old turns. Resending the full transcript every call blows the window and the bill |
| Multi-turn RetrieveAndGenerate | Pass `sessionId`. Omitting it makes every turn stateless |
| A clarifying-question loop | Step Functions (wait, branch) |
| Intent of one utterance | The model, or Comprehend when you want a deterministic entity API |

## Caching the prefix

Stable bytes come first: system prompt, tool schemas, few-shot block, a static policy. Place a `cachePoint` after that prefix. Dynamic bytes (the user message, today’s date, retrieved chunks that change every call) come after the marker.

Any byte change in the prefix misses: timestamp, whitespace, JSON key order, an id. A timestamp at the top of the system prompt makes the cache useless.

Below the model’s minimum token threshold the marker is ignored and both cache metrics stay zero. That is expected, not an error.

## Quality

- Diff prompt versions the way you diff code.
- A regression set covers the happy path, edges, injection, empty retrieval, and schema failures. Run it when the prompt or the model changes.
- A prompt change without an eval run is a guess.
- Lambda (or the handler) asserts the schema before the response reaches the user.

Guardrails enforce policy the prompt cannot be trusted to enforce. A system prompt and a guardrail do different jobs. Use both. See `safety.md`.
