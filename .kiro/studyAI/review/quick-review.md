# Quick review

One pass the night before. Details live in `domains/` and `reference/`. Pass mark is **750**. 65 of 75 questions count. No penalty for guessing. Domains **31 / 26 / 20 / 12 / 11**. Compensatory scoring.

## Pattern picker

| If the stem needs | Choose |
|---|---|
| Current private facts, citations | RAG, not fine-tune |
| Stable style or format | Prompt, then fine-tune / LoRA |
| Model picks tools | Agent |
| Steps already known | Step Functions or Prompt Flows |
| Person must approve a side effect | Step Functions callback or A2I |
| Least ops, variable traffic | Bedrock on-demand + Lambda |
| Steady high token volume | Provisioned throughput |
| Overnight bulk | Batch inference |
| Data stays in-geography | `eu.` / `us.` / `apac.` profile, never `global.` |
| Swap model without a deploy | AppConfig + gateway |
| Employees on company data | Q Business |
| Developers writing code | Q Developer |
| PII in chat | Guardrail BLOCK or ANONYMIZE |
| PII in S3 | Macie |
| PII in a string you process | Comprehend |
| Hallucination | RAG + citations + grounding threshold |
| Toxicity / jailbreak | Guardrail content and prompt-attack filters |
| Who called the API | CloudTrail **data events** |
| What was in the prompt | Invocation logs (encrypt them) |
| Cost by team | Tagged inference profile + Cost Explorer |
| Spiky and throttling | Queue, `maxTokens`, cross-Region profile |
| Same huge prefix every call | Prompt cache, stable bytes first |

## Domain 1 in twelve lines

1. PoC on Bedrock before a platform. Well-Architected GenAI Lens for standards.
2. Small model for simple and cheap. Large model for hard reasoning. Titan Embeddings V2 to embed. Match modality.
3. Inference profiles: geo prefix stays in geo. IAM needs profile ARN **and** `arn:aws:bedrock:*::foundation-model/...`.
4. Custom model lifecycle is registry → approve → deploy → canary → rollback. Training theory is out of scope.
5. Glue Data Quality and Wrangler validate **data**. They do not score answers.
6. Format Converse as user/assistant turns plus a system field. InvokeModel bodies are per provider.
7. Filter metadata before vector search. Sidecar file is `name.ext.metadata.json`.
8. OpenSearch for hybrid and filters. pgvector if Aurora is already there. Neptune for graphs. DynamoDB is not the index.
9. Chunking is immutable: fixed (FAQ), semantic (long prose), hierarchical (headings), none (pre-split). Tables need a real parser.
10. Hybrid when the query has IDs or error codes. Rerank when order is wrong. Decompose multi-hop questions.
11. One embedding model per index. Change the model, rebuild the index.
12. Prompt Management for versions. Prompt Flows for branches. Cache the stable prefix.

## Domain 2 in twelve lines

1. Action group = OpenAPI + Lambda, or return of control.
2. `PrepareAgent`, then move the alias. `DRAFT` is not production.
3. AgentCore Runtime hosts your loop. Gateway turns REST into MCP. Harness is the managed config loop.
4. Strands = agent SDK. Agent Squad = router plus specialists.
5. Stateless MCP → Lambda. Heavy MCP → ECS.
6. Lambda + on-demand for spiky calls. SageMaker when you bring weights. Containers when memory and GPU are yours.
7. Cascade easy traffic to a small model.
8. Sync = API Gateway. Tokens on screen = ConverseStream (SDK, not CLI) + WebSocket or response streaming. Later = SQS or EventBridge.
9. Set `maxTokens`. Adaptive retry. Do not retry access denied or validation errors.
10. PrivateLink for the VPC path. Outposts if data cannot leave. Wavelength only for real edge latency.
11. Gateway in front so apps do not hold Bedrock credentials.
12. Q Developer ≠ Q Business. CodeGuru is out of scope.

## Domain 3 in ten lines

1. Layers: WAF → pre-filter → guardrail → grounding → schema check → human.
2. Content filters do not fix lies. Grounding and RAG do.
3. `BLOCK` vs `ANONYMIZE` per PII type.
4. Three apply modes: `guardrailConfig`, `guardContent`, `ApplyGuardrail`.
5. Word filters ignore `guardContent` boundaries. Other filters honor them. Wrap tool output and chunks.
6. Stream `sync` for compliance. `async` can leak and will not anonymize.
7. Pin a numbered guardrail version. Force it with `bedrock:GuardrailIdentifier`.
8. Anonymize does not scrub invocation logs. KMS, IAM, retention, Macie.
9. Model cards, Glue lineage, citations, CloudTrail. That is governance.
10. Fairness is a sliced eval plus a gate. A2I is the human on the live path.

## Domain 4 in eight lines

1. Bill ≈ input tokens + output tokens. Output usually costs more. Cap both.
2. Prompt cache is exact prefix match. Timestamps above the cache point are a miss. Under the token minimum is a silent miss.
3. Semantic cache for paraphrased public FAQs. Not for per-user facts.
4. Provisioned throughput only when it stays busy. Batch when nobody is waiting.
5. Priority tier or streaming for time-to-first-token. Precompute the questions you can predict.
6. Alarm on throttles, token bursts, latency, grounding failures, tool loops, cost anomalies.
7. Cost Explorer and Cost Anomaly Detection are in scope. Budgets and Savings Plans are not.
8. Scale on tokens and GPU memory, not CPU, for model workloads.

## Domain 5 in eight lines

1. Do not pick a chatbot with BLEU or ROUGE.
2. RAG quartet: context recall, context precision, faithfulness, answer relevance.
3. Retrieval metrics: recall@k, precision@k, MRR, nDCG.
4. Agents: task success **and** tool choice. Lucky wrong-tool completions fail the rubric.
5. LLM-as-judge needs a human sample before it is a release gate.
6. Golden set on every prompt change. Canary the alias. Roll back on a drop.
7. Low-QPS throttle → `maxTokens` and quota, not a reranker.
8. Right doc, wrong span → chunking. Doc never appears → ingest, filter, or embedding mismatch.

## Numbers and names

| Item | Value |
|---|---|
| Pass | 750 / 1000 |
| Questions | 75 total, 65 scored, 10 unmarked unscored |
| Time | 180 minutes |
| Grounding start point | ~0.7, then tune |
| Fixed-chunk overlap | about 10–20% |
| Guardrail in prod | numbered version, trace disabled |
| Cache TTLs | 5 min default, 1 hour on specific Claude models |
| AgentCore session id | long (33+ characters) |
| Retry mode | adaptive |

## Error → first move

| Error | First move |
|---|---|
| `ThrottlingException` | Set `maxTokens`, check token quota, then profile or queue |
| `AccessDeniedException` | IAM action and ARN, SCP, streaming permission, cross-Region wildcard |
| `ValidationException` | Body, inference profile vs on-demand, window, hybrid override |
| Malformed input | Wrong provider JSON. Use Converse |
| Old agent behavior | `PrepareAgent` and the alias |
| PII in logs | Encrypt or disable invocation logging |
| Empty KB | Filter, ingestion job, embedding mismatch |
| Hung agent | Return of control waiting for a tool result |

## Service swaps you will see

| Wrong | Right |
|---|---|
| SES | SNS |
| MQ | SQS or EventBridge |
| Redshift | Athena or Aurora pgvector |
| CodeGuru | Q Developer |
| Budgets | Cost Explorer or Cost Anomaly Detection |
| Trusted Advisor | CloudWatch and cost anomaly |
| Beanstalk / Batch / Lightsail | Lambda, ECS, SageMaker, Bedrock |
| Macie for a live prompt | Guardrail or Comprehend |
| Comprehend for a bucket scan | Macie |
| VPN / Direct Connect as the safety control | PrivateLink |
| DynamoDB as ANN | OpenSearch, pgvector, KB |
| Fine-tune for weekly policy | RAG |

## Multi-select habit

If the question says “select **two**” or “select **three**,” each choice must be necessary. Pairs that often both belong:

- Guardrail **and** encrypted invocation logs
- Metadata filter **and** hybrid search
- Recall@k **and** faithfulness
- `InvokeModel` **and** `InvokeModelWithResponseStream`
- Inference profile ARN **and** wildcard-region model ARN
- Numbered guardrail **and** IAM condition
- Step cap **and** tool IAM boundary

Drop any option whose service is out of scope, even if the sentence is well written.
