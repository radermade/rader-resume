# Domain 4 — Operational efficiency and optimization

**12% of scored content.** Tasks 4.1–4.3. Same architecture, lower bill, lower latency, and a dashboard that shows token, quality, and cost problems. If the stem is about correctness of facts, go to Domain 5 or RAG. If it is about dollars, milliseconds, or “we cannot see it,” stay here.

## 4.1 Cost and resource efficiency

Price is roughly **input tokens + output tokens**, and output is usually more expensive. Cached input reads are cheaper and do not count the same way against throughput quotas. Do not memorize dollar amounts.

### Token efficiency

| Lever | Effect |
|---|---|
| Shorter system prompt and fewer few-shot examples | Fewer input tokens every call |
| Prompt caching | Reuse a stable prefix (system, tools, long docs) |
| Retrieve fewer, better chunks | Less context, better answers |
| Cap `maxTokens` | Stops runaway completions **and** stops quota over-reservation |
| Ask for concise output | Cuts output tokens |
| Compress or summarize history | Conversation cost stays flat |
| Drop irrelevant tool definitions | Large tool schemas are input tokens |

### Which model and which billing mode

| Mode | Pay for | Fit |
|---|---|---|
| On-demand | Tokens used | Default, spiky, uncertain volume |
| Service tier: Priority / Standard / Flex | Same on-demand family, different latency vs price | Priority when waiting is expensive. Flex when it can queue |
| Batch inference | Discounted async | Thousands of documents, no user waiting |
| Provisioned throughput | Reserved model units, commit | Steady high utilization. Idle commitment is wasted money |
| Reserved tier | 1- or 3-month capacity, uptime target | Mission-critical, cannot sit on shared on-demand |
| Cross-Region inference | On-demand rates, more burst headroom | Throttling and Regional availability, not a discount |
| Cascade / routing | Cheap model for easy traffic | Most calls do not need the flagship model |
| Prompt caching | Reduced input price on hits | Large **stable** prefix, repeated |

Provisioned throughput wins only when utilization is high enough that on-demand would cost more. Low QPS + provisioned = the trap answer.

### Caching layers (do not mix them up)

| Cache | Key | Misses when |
|---|---|---|
| Prompt cache (Bedrock) | Exact prefix before a `cachePoint` | Any byte change: timestamp, whitespace, JSON key order, id in the prefix |
| Exact result cache | Hash of the full request | Wording changes at all |
| Semantic cache | Embedding similarity of the question | Threshold too high (never hits) or too low (wrong answer reused) |
| Edge cache (CloudFront, Lambda@Edge, ElastiCache) | HTTP or app key | Personalized or tenant-specific answers cached globally |

Good prompt-cache candidates: system prompt, tool schemas, few-shot block, a static policy. Bad: the user message, “today is {date}” at the top, retrieved chunks that change every call. Put stable bytes **first**, cache point, dynamic bytes after.

TTL: 5 minutes is the broad default. 1 hour is for specific Claude models and stable prompts. Longer TTL must be placed before a shorter one when both are used.

Below the model’s minimum token threshold, the cache point is ignored and both cache metrics stay zero. That is not an error.

### Throughput capacity

- Measure **tokens per minute**, not only requests per second. A few huge prompts throttle a busy API of tiny prompts.
- Batch embeddings and batch inference for bulk. Do not parallelize without a concurrency cap; you will throttle yourself.
- Autoscale SageMaker or ECS on **GPU memory and in-flight tokens**, not only on CPU.
- Quota: Service Quotas for the model. A throttle with quota headroom is often `maxTokens` reservation or a retry storm.

**Pick this when:** “same 8k system prompt on every chat turn” → prompt caching. “FAQ asked in many wordings” → semantic cache. “50,000 PDFs tonight” → batch inference. “steady 80% of a model’s throughput all day” → provisioned. “traffic varies 100x” → on-demand, maybe with a queue.

## 4.2 Performance

| Symptom | First lever |
|---|---|
| Time to first token matters | `ConverseStream`, latency-optimized model or Priority tier |
| Full answer is slow | Smaller model, shorter output, fewer tools, fewer chunks |
| Predictable questions | Precompute overnight and serve the cache |
| Independent subtasks | Parallel model calls, then merge. Not one giant prompt |
| Retrieval is the slow part | Index tuning, smaller embedding, metadata filter to cut the search space, hybrid with a tight `k` |
| Retrieval is the wrong part | Query rewrite, rerank, chunking. Speed is not the issue |
| GPU endpoint queue | Batch size, instance size, autoscaling, Neo only if it is a compiled classic model |
| Parameters | Lower temperature for stable tasks. top-p / top-k only if the stem is about diversity. They are not a latency strategy |

Latency budget example the questions imply: retrieve (filter + kNN + optional rerank) + time to first token + output tokens. Streaming hides output time. It does not hide retrieval time. Precompute hides both for known queries.

A/B test parameter and prompt changes against the eval set. “We raised temperature” is not an optimization story.

**Pick this when:** “users stare at a blank page” → streaming. “p95 is retrieval” → index, filter, cache embeddings of hot queries. “p95 is output tokens” → shorter answers or a faster model. “same dashboard every morning” → precompute.

## 4.3 Monitoring

Log the genAI signals. CPU of the Lambda is not enough.

### Built-in and custom

| Signal | Where |
|---|---|
| Invocations, latency, errors, throttles | CloudWatch Bedrock runtime metrics |
| Input and output token counts | CloudWatch and the `usage` block. Alarm on token bursts |
| Prompt and completion bodies | Model invocation logging (S3 and/or CloudWatch Logs). Sensitive. Encrypt |
| Cache hit | `cacheReadInputTokens` vs `cacheWriteInputTokens` |
| Trace across services | X-Ray. Also agent traces for tool steps |
| Quality | Custom metrics: thumbs, grounding failures, schema failures, hallucination samples |
| Cost | Cost Explorer by usage type and tags. Cost Anomaly Detection. Application inference profiles so a tag can split teams |
| Business | Custom dashboard: containment rate, tickets deflected, revenue per 1k tokens |

Invocation logging is **not** on by default in a useful way for forensics until you enable it. CloudTrail data events show the API call, not a readable prompt, unless you also have invocation logs.

### What to alarm on

- Throttles and 5xx.
- Sudden input-token jump (prompt regression or a loop).
- Output-token jump (runaway `maxTokens`).
- Latency p95.
- Quality: drop in user score, rise in “I don’t know,” rise in grounding blocks.
- Cost anomaly.
- Retrieval: empty-result rate, latency, index errors.
- Tools: error rate, timeout rate, calls per task (a loop).

### Vector store ops

Monitor search latency, rejection (filter matches nothing), index size vs document count (ingestion stuck), and refresh lag (source newer than index). Reindex after an embedding-model change. Validate a sample of chunks after parsing changes.

### GenAI-specific debug hooks

| Hook | Shows |
|---|---|
| Golden set replay | Did this deploy shift answers |
| Output diff | Same prompt, two versions |
| Reasoning / agent trace | Which tool, which thought, where it went wrong |
| Correlation id | API Gateway → Lambda → Bedrock request id in one log line |

Managed Grafana is in scope if the stem wants a cross-account dashboard on top of CloudWatch. QuickSight is in scope for business reporting of those metrics. Neither replaces invocation logs.

**Pick this when:** “which prompt version caused the spike” → invocation logs with the prompt id, Logs Insights. “which team spent this” → tagged inference profile + Cost Explorer. “did the new chunking help” → retrieval metrics and an eval, not GPU utilization. “agent calls the refund tool 20 times” → tool-call metrics and a step limit.

## Domain 4 traps

- Provisioned throughput for a spiky proof of concept.
- Prompt caching with a timestamp in the cached prefix.
- Semantic cache for questions whose correct answer changes per user.
- Batch inference for a chat UI.
- CloudFront caching personalized completions.
- “Turn on AWS Budgets.” Budgets, Cost and Usage Report, and Savings Plans are **out of scope**. Cost Explorer and Cost Anomaly Detection are in scope.
- Raising temperature to reduce latency.
- Scaling on CPU for a token-bound workload.
- Caching the model response in front of a guardrail so a blocked category gets served from cache. Cache **after** policy, and include tenant and policy version in the key.
