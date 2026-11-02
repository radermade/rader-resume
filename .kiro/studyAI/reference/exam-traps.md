# Exam traps

Distractors that are real AWS services used for the wrong job. If two answers “work,” pick the one that matches the constraint with the least machinery.

## Pattern traps

| Tempting answer | Why it loses | Pick instead |
|---|---|---|
| Fine-tune on the company wiki | Facts go stale. Retrain cycle | RAG / Knowledge Base |
| RAG to change brand voice | Retrieval will not stably change style | Prompt or fine-tune |
| Agent with no tools | Loop overhead, less control | Converse, or Step Functions if steps are fixed |
| Bigger model for bad retrieval | Fluency up, facts still missing | Fix chunking, filters, hybrid, rerank |
| Lower temperature to stop hallucinations | Less random, still ungrounded | Grounding + “say when not in context” |
| Content filter to stop hallucinations | Filters toxicity, not truth | Contextual grounding or RAG |
| BLEU to choose a chatbot model | N-gram overlap ≠ helpful or true | Faithfulness, relevance, human or LLM-judge |
| One “accuracy” for RAG | Hides the broken stage | Split retrieval and generation |
| String match in CI for paragraphs | Flaky forever | Schema, rubric, semantic check, golden-set judge |
| Retry AccessDenied / ValidationException | Never succeeds | Fix IAM or the body |
| `global.` profile for EU data | Leaves the geography | `eu.` profile and EU Regions |
| Provisioned throughput for a spiky POC | Pay for idle | On-demand |
| On-demand for a custom model that requires PT | Will not run | Provisioned throughput |
| Batch inference for chat | Not interactive | On-demand streaming |
| Prompt cache with a date in the prefix | Cache never hits | Move dynamic fields below the cache point |
| Semantic cache for per-user account answers | Serves another user’s fact | Exact cache keyed by user, or no cache |
| CloudFront cache of personalized answers | Cross-user leak | Cache only public, identical completions |
| Guardrail anonymize = logs are clean | Logs still have raw PII | KMS on logs, IAM, retention, Macie, or disable invocation logging |
| Async guardrail streaming for HIPAA | PII can flush before the check. No anonymize | `sync` streaming mode |
| System prompt as the only safety control | Not enforceable | Guardrail + IAM condition |
| `DRAFT` guardrail or agent alias in prod | Mutable, surprise changes | Numbered version and an alias |
| Guardrail id without version | Often not applied | Both fields |
| `trace: enabled` in prod | Echoes the matched PII | `disabled` |
| Metadata filter described only in the prompt | Model can ignore it. Data was already retrieved | Filter at query time |
| Hybrid search on a store that is semantic-only | API rejects or ignores | OpenSearch, Aurora/RDS, or Mongo with a text field |
| New embedding model, old index | Dimensions or spaces do not match | Full re-embed |
| Change chunking on a live data source | Immutable | New data source, re-ingest |
| DynamoDB as the vector index | It stores metadata and keys | OpenSearch, pgvector, S3 Vectors, KB |
| Kendra and “just use embeddings” as the same answer | Different products | Kendra for managed enterprise search; KB/OpenSearch when you own the pipeline |
| Q Developer for employee wiki search | Wrong Q | Q Business |
| Q Business for IDE code generation | Wrong Q | Q Developer |
| CodeGuru for genAI coding help | Out of scope | Q Developer |
| Macie to mask PII in a live chat | Macie scans S3 | Guardrail or Comprehend |
| Comprehend to discover PII across a data lake | Wrong tool | Macie |
| Glue Data Quality to score hallucinations | It scores datasets | Eval job / grounding |
| SageMaker Clarify as the genAI answer quality tool | Classic model bias | LLM-judge, human review, Bedrock evaluations. Clarify if the stem is a classic model |
| Human approval via “a better prompt” | Not a control | A2I or Step Functions callback |
| CloudTrail management events as the prompt archive | Invoke is a data event. Body is in invocation logs | Data events + model invocation logging |
| Budgets or Savings Plans for the cost answer | Out of scope on this exam | Cost Explorer, Cost Anomaly Detection, tags on inference profiles |
| Amazon MQ between services | Out of scope | SQS, SNS, EventBridge |
| SES to notify | Out of scope | SNS |
| Elastic Beanstalk or Batch as the default host | Out of scope | Lambda, ECS/EKS, SageMaker, Bedrock |
| Redshift as the vector store | Out of scope | Aurora pgvector, OpenSearch |
| Trusted Advisor to monitor token spend | Out of scope | CloudWatch + Cost Anomaly Detection |
| Raise temperature to cut latency | Unrelated | Smaller model, streaming, fewer tokens |
| Scale Lambda CPU to speed Bedrock | The wait is the model | Streaming, smaller model, provisioned, shorter prompts |
| Forgot `InvokeModelWithResponseStream` in IAM | Sync works, stream denied | Both actions |
| Cross-Region profile IAM only on one Region’s model ARN | Routed Region denied | Wildcard Region on the foundation-model ARN |
| `PrepareAgent` skipped | Old instructions still live | Prepare, then point the alias at the new version |
| Two user messages in a row | Validation error | Alternate user and assistant |
| Titan JSON sent to Claude InvokeModel | Malformed input | Converse, or the right body |
| Unset `maxTokens` | Quota reserved at the model max | Set it |
| Sessionless `RetrieveAndGenerate` | Every turn forgets the chat | Pass `sessionId` |
| Word filter expected to honor `guardContent` | Word filters scan all text | Know which filters are selective |

## Reading the stem

- “Least operational overhead” → managed: Bedrock, managed KB, on-demand, Prompt Management, Q Business. Not a SageMaker cluster you patch.
- “Must not change application code” → AppConfig, alias, inference profile, gateway.
- “Already uses Aurora” → pgvector, not a new OpenSearch domain, unless they also need features Aurora cannot do.
- “Exact identifier and natural language” → hybrid.
- “Show sources” → citations from RetrieveAndGenerate or an agent KB, not a higher temperature.
- “Regulator / residency” → geo inference profile, KMS CMK, private endpoint, log retention. Not `global.`.
- “Spike lasted five minutes” → on-demand, queue, or caching. Not a one-year provisioned commitment.
- “Multiple correct” questions: a single control is often incomplete. Safety + logging, or retrieval + generation metric, are the usual pairs. Select every option that is required, and none that are merely nice.

## Out-of-scope bait

If the answer’s main service is on the out-of-scope list, it is wrong even if the architecture is clever. Frequent bait: Budgets, CUR, Savings Plans, SES, MQ, Redshift, CodeGuru, Beanstalk, Batch, Lightsail, Trusted Advisor, Forecast, Fraud Detector, DevOps Guru, Direct Connect, Transit Gateway, VPN, DeepRacer. Full list: [in-scope-and-out-of-scope.md](in-scope-and-out-of-scope.md).
