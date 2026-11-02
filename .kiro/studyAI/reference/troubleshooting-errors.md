# Troubleshooting errors

Match the exception before changing the architecture.

## Retry or not

| Retry with backoff | Do not retry |
|---|---|
| `ThrottlingException` | `ValidationException` |
| `ModelTimeoutException` | `AccessDeniedException` |
| `ServiceUnavailableException` | `ResourceNotFoundException` |
| `InternalServerException` | Malformed input (it is a validation failure) |

SDK: adaptive retry mode, limited attempts, jitter. A hand-rolled tight loop makes throttling worse.

## Exceptions

### `ThrottlingException`

1. Is `maxTokens` set? If not, the call reserves the model’s maximum output.
2. Compare CloudWatch token usage with Service Quotas for that model.
3. Spread load with a cross-Region inference profile inside the allowed geography.
4. Smooth bursts with SQS.
5. If utilization is steadily at the cap, provisioned throughput.
6. Check for a retry storm.

### `AccessDeniedException`

| Cause | Clue |
|---|---|
| Missing `bedrock:InvokeModel` | Sync fails |
| Missing `bedrock:InvokeModelWithResponseStream` | Only streaming fails |
| ARN is the model but the call uses an inference profile | Fix the resource element |
| Cross-Region profile without `bedrock:*::foundation-model/...` | Fails when routed out of the home Region |
| SCP denies that Region or model | Works in another account or Region |
| Anthropic use-case form not submitted | First call, Anthropic only |
| Marketplace subscribe permission missing | Third-party model, first invoke |
| New IAM role | Retry after propagation. Then it is not this |
| Expired credentials | Everything fails at once |
| VPC endpoint policy | Works from a laptop, fails from the subnet |

### `ValidationException` / malformed input

- InvokeModel body for the wrong provider.
- On-demand not supported for that model. Use the inference profile or provisioned ARN.
- `maxTokens` above the model cap, or input too long.
- Tool schema invalid.
- Prefill (assistant message ending the request) on Claude versions that removed it.
- Guardrail version `DRAFT` where a published version is required, or a bad id.
- Hybrid search override on a store that cannot do it.
- Embedding dimension ≠ index dimension.

### `ResourceNotFoundException`

Wrong Region, deleted alias, model id typo, or the KB / agent is in another account and the ARN was not shared.

### Timeout

Model is still generating (lower `maxTokens` or stream), Lambda hit its own limit while waiting (increase Lambda timeout or move to async), or a tool hung (tool timeout and circuit breaker). Cold start of a SageMaker or ECS container is a different graph: provisioned concurrency or min capacity, not a Bedrock quota.

### `UnknownOperationException`

The API was sent to the wrong endpoint. `Converse` → `bedrock-runtime`. `Retrieve` → `bedrock-agent-runtime`. `PrepareAgent` → `bedrock-agent`.

## Behavior bugs (no exception)

| What you see | Cause |
|---|---|
| Old instructions after an edit | `PrepareAgent` not run, or alias still on the old version |
| Guardrail never fires | Missing version, not passed on `RetrieveAndGenerate`, or IAM does not require it and the client omitted it |
| PII still in CloudWatch | Invocation logging stores the raw body. Anonymize is response-only |
| Cache metrics always zero | Under token minimum, unsupported model, or prefix not byte-identical |
| Multi-turn KB forgets context | No `sessionId` |
| Agent hangs | Return of control with no tool result posted |
| Empty retrieval | Filter, failed ingestion, wrong embedding, or threshold |
| Answers from deleted docs | Deletion policy retain, or sync lag |
| Stream cut off | `max_tokens` stop reason. Raise the cap you intentionally set |
| JSON occasionally invalid | Temperature, no schema check, no retry-on-parse |
| High latency to first byte | Not streaming, or time is spent in retrieval before the model starts |
| Works in console, fails in app | IAM on the role, or the console uses your user and the app uses a role without the profile ARN |
| Quality drop after a “no change” deploy | Prompt template variable, AppConfig, or chunking job picked up new documents |

## Context window

Symptoms: error naming the limit, or the model “forgets” the middle of the prompt.

Order of cuts:

1. Tool outputs (truncate in the tool).
2. History (summarize).
3. Retrieved chunks (lower k, rerank, hierarchical parent only when needed).
4. Few-shot examples (cache the ones you keep).
5. Only then, a model with a larger window (more money).

## A short debug order

1. Request id, model id, Region, prompt version, agent alias.
2. X-Ray: time in retrieval vs model vs tool.
3. Stop reason and token usage.
4. Invocation log for the actual prompt (handle as sensitive).
5. Guardrail trace in a non-prod call.
6. Agent trace for tool choice.
7. Retrieval hits for the same question in isolation.
8. Compare to the last golden-set run.

Do not change temperature, chunking, and the model in one deploy. You will not know which fix worked.
