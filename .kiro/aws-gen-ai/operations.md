---
inclusion: auto
name: aws-gen-ai-operations
description: "Use when optimizing, monitoring, evaluating, or troubleshooting a generative AI system on AWS: token cost, prompt caching, batch and provisioned throughput, latency, CloudWatch, eval metrics, RAG failure split, and Bedrock errors."
---

# Operations

Price is input tokens plus output tokens, and output usually costs more. Cached input reads are cheaper and do not count the same way against throughput quotas. Measure tokens per minute, not only requests per second. Verify current prices in the account. The levers below are stable.

## Token and capacity levers

| Lever | Effect |
|---|---|
| Shorter system prompt, fewer few-shot examples | Fewer input tokens on every call |
| Prompt cache on a stable prefix | Cheaper repeated prefix, less quota burn |
| Fewer, better chunks | Less context and better answers |
| `maxTokens` set | Stops runaway completions and quota over-reservation |
| Concise-output instruction | Cuts output tokens |
| Summarized history | Conversation cost stays flat |
| Fewer tool definitions | Large schemas are input tokens |
| Route easy traffic to a small model | The flagship model handles the hard slice |
| Exact cache | Identical requests cost nothing the second time |
| Semantic cache | Paraphrases of a shared answer hit. Threshold too low serves the wrong answer. Per-user facts stay out |
| Batch inference | Overnight volume, nobody waiting |
| Provisioned throughput | Steady high utilization where the commit beats on-demand |
| On-demand | Spiky or unknown volume. The default |
| Priority / Standard / Flex | Same on-demand family. Priority when waiting is expensive. Flex when the work can queue |
| Cross-Region inference | More burst inside a geography. It is not a discount |

Batch embeddings and batch inference for bulk work. Cap concurrency. Unbounded parallel calls throttle themselves.

Prompt-cache candidates: system prompt, tool schemas, few-shot block, a static policy. Poor candidates: the user message, a date at the top, retrieved chunks that change every call. Stable bytes first, cache point, dynamic bytes after.

CloudFront may cache public, non-personalized answers. Personalized or tenant-specific completions are not an edge-cache key by themselves. Cache after the policy check, and include tenant and policy version in the key. A cached response must not bypass a guardrail.

## Latency

| Symptom | First lever |
|---|---|
| Time to first token | `ConverseStream`, a faster model, or the Priority tier |
| The full answer is slow | Smaller model, shorter output, fewer tools, fewer chunks |
| The questions are known | Precompute and serve the cache |
| Subtasks are independent | Parallel calls, then merge |
| Retrieval dominates | Index, metadata filter, a tighter `k`, cached embeddings of hot queries |
| Retrieval returns the wrong text | Query rewrite, rerank, chunking. Speed is a different bug |
| GPU endpoint queue | Batch size, instance size, autoscaling. Neo only for a compiled classic model |

A latency budget is retrieve (filter, kNN, optional rerank) plus time to first token plus output tokens. Parameter changes go through the eval set.

## What to record and alarm

CPU of the Lambda is not the genAI signal.

| Signal | Where |
|---|---|
| Invocations, latency, errors, throttles | CloudWatch Bedrock runtime metrics |
| Input and output tokens | The `usage` block and CloudWatch. Alarm on bursts |
| Prompt and completion bodies | Invocation logging to S3, CloudWatch Logs, or both. Sensitive. Encrypt |
| Cache hit | `cacheReadInputTokens` against `cacheWriteInputTokens` |
| Trace | X-Ray across API Gateway, Lambda, Bedrock, and tools. Agent trace for tool steps |
| Quality | Custom metrics: thumbs, grounding failures, schema failures, empty retrieval |
| Cost | Cost Explorer by usage type and tags. Cost Anomaly Detection for spikes. Application inference profiles so a tag splits teams |
| Business | Containment, tickets deflected, revenue per thousand tokens |

Invocation logging is off until you enable it. CloudTrail data events show that the API was called. They do not contain a readable prompt.

Alarm on throttles and 5xx, a sudden input-token jump (prompt regression or a loop), an output-token jump, latency p95, a drop in user score, a rise in grounding blocks, cost anomalies, empty-result rate, retrieval latency, and tool error rate or calls per task.

Vector indexes: search latency, filters that match nothing, index size versus document count (ingestion stuck), and refresh lag. Validate a sample of chunks after a parsing change.

Correlate one id from API Gateway through Lambda to the Bedrock request id. Managed Grafana fits a cross-account operational dashboard. QuickSight fits a business report of the same numbers. Neither replaces invocation logs.

## What to measure

Open-ended generation has no single accuracy. Name the layer.

| Layer | Measure |
|---|---|
| Retrieval | Recall@k, precision@k, MRR, nDCG, hit rate, empty-result rate, search latency |
| Generation | Relevance, faithfulness, consistency, fluency, completeness |
| Retrieval plus generation | Faithfulness, answer relevance, context precision, context recall, separately |
| Safety | Toxicity, PII leakage, jailbreak success, false refusals |
| Agent | Task success, tool-choice accuracy, argument accuracy, steps per task, tool error rate, confirmations that were required |
| Cost and speed | Tokens per task, latency to first token, price per successful task |
| People | Thumbs, a rubric, Ground Truth for a labeled set, Augmented AI for a live queue |

BLEU and ROUGE fit translation and some summaries. They are weak primary scores for chat. Exact match fits labels and short spans. Semantic similarity can score a fluent wrong answer as close. Temperature is a parameter, not a metric.

The four numbers that locate a retrieval failure:

1. **Context recall.** The needed facts were retrieved.
2. **Context precision.** They were not buried in junk.
3. **Faithfulness.** The answer sticks to that context.
4. **Answer relevance.** The answer addresses the question.

| Context | Answer follows it | Answer ignores it |
|---|---|---|
| Relevant | Working retrieval | Fix the prompt or add grounding |
| Not relevant | Fix retrieval. The model was loyal to junk | Fix both |

A high task-completion rate with a forbidden tool is a failed agent run when the rubric names the tool.

Bedrock model evaluation compares models and prompts on your dataset (automatic metrics, a judge model, human workers). Agent evaluation jobs score task success. A judge model is cheap and biased toward its own style. Calibrate it on a human sample before it gates a release. High-stakes tone and policy stay with people.

Put cost next to quality. A model that is slightly more faithful and several times the tokens is a business call. Freeze the dataset you report. Record model id, prompt version, temperature, `k`, chunk config, and date. Keep safety regressions separate from quality regressions.

## Troubleshooting order

Work outside in: quota and IAM, then request shape, then prompt, then retrieval, then the model. Change one variable per deploy.

Debug with the request id, model id, Region, prompt version, and agent alias. Then X-Ray (time in retrieval versus model versus tool), stop reason and token usage, the invocation log (handle it as sensitive), a non-prod guardrail trace, the agent trace, retrieval hits for the same question alone, and the last golden-set run.

### Retry or not

| Retry with backoff and jitter | Do not retry |
|---|---|
| `ThrottlingException` | `ValidationException` |
| `ModelTimeoutException` | `AccessDeniedException` |
| `ServiceUnavailableException` | `ResourceNotFoundException` |
| `InternalServerException` | Malformed input |

### Exceptions

**`ThrottlingException`.** Set `maxTokens`. Compare CloudWatch token usage with Service Quotas. Spread burst with a cross-Region profile inside the geography. Smooth with SQS. Move to provisioned throughput when utilization sits at the cap. Look for a retry storm.

**`AccessDeniedException`.** Missing `InvokeModel`, or missing `InvokeModelWithResponseStream` when only streaming fails. The resource is the model ARN while the call uses an inference profile. Cross-Region without the wildcard-Region foundation-model ARN. An SCP. The Anthropic use-case form on a first Anthropic call. Marketplace permission on a first third-party invoke. A new role that has not propagated yet (retry once for that, then look elsewhere). Expired credentials. A VPC endpoint policy.

**`ValidationException`.** InvokeModel body for the wrong provider. On-demand not supported (use the inference profile or provisioned ARN). `maxTokens` or input over the model cap. Invalid tool schema. Assistant prefill on a Claude model that removed it. Guardrail id or `DRAFT` where a published version is required. Hybrid search on a store that cannot do it. Embedding dimension different from the index.

**`ResourceNotFoundException`.** Wrong Region, deleted alias, model id typo, or a knowledge base or agent in another account whose ARN was not shared.

**`UnknownOperationException`.** The API went to the wrong endpoint. See `apis.md`.

**Timeout.** Output too long (lower `maxTokens` or stream), Lambda’s own timeout while it waits, or a hung tool. A SageMaker or ECS cold start is min capacity, not a Bedrock quota.

### No exception

| What you see | Cause |
|---|---|
| Old instructions after an edit | `PrepareAgent` was not run, or the alias still points at the old version |
| Guardrail never fires | Missing version, omitted on `RetrieveAndGenerate`, or the client skipped it and IAM does not require it |
| PII in CloudWatch | Invocation logs store the raw body. Anonymize affects the response |
| Cache metrics always zero | Under the token minimum, unsupported model, or the prefix is not byte-identical |
| Multi-turn retrieval forgets the conversation | No `sessionId` |
| Agent hangs | Return of control, and no tool result was posted |
| Empty retrieval | Filter, failed ingestion, wrong embedding, or the threshold |
| Deleted documents are still cited | Deletion policy retained them, or sync is behind |
| Stream cut off | Stop reason `max_tokens`. Raise the cap you set on purpose |
| JSON sometimes invalid | Temperature, no schema check, no parse-failure path |
| Slow time to first byte | Not streaming, or retrieval runs before the model starts |
| Console works, the app fails | The console uses your user. The app role is missing the profile ARN or the stream action |
| Quality drop after a “no change” deploy | A prompt variable, AppConfig, or an ingestion job picked up new documents |

### Context window

Symptoms: an error that names the limit, or the model drops the middle of the prompt. Cut in this order:

1. Tool outputs, truncated in the tool.
2. History, summarized.
3. Retrieved chunks: lower `k`, rerank, return the parent only when the section is required.
4. Few-shot examples. Cache the ones you keep.
5. A larger window, which costs more, only after the above.
