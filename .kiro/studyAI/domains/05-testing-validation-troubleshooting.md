# Domain 5 — Testing, validation, and troubleshooting

**11% of scored content.** Tasks 5.1–5.2. Small weight, easy points if you can name the metric and the failure mode. Metric definitions: [evaluation-metrics.md](../reference/evaluation-metrics.md). Error catalog: [troubleshooting-errors.md](../reference/troubleshooting-errors.md).

## 5.1 Evaluation

Traditional accuracy, BLEU, and ROUGE are weak primary scores for open-ended answers. The exam wants task-shaped metrics.

### What to measure

| Layer | Questions | Metrics |
|---|---|---|
| Retrieval | Did we fetch the right text? | Recall@k, precision@k, MRR, nDCG, hit rate, empty-result rate, latency |
| Generation | Is the answer good? | Relevance, faithfulness / groundedness, consistency, fluency, completeness |
| RAG end to end | Is the answer good **because of** the context? | Faithfulness, answer relevance, context precision, context recall |
| Safety | Does it refuse or leak? | Toxicity rate, PII leakage, jailbreak success rate |
| Agent | Did the job finish? | Task success, tool-choice accuracy, steps per task, tool error rate |
| Cost / speed | Can we afford it? | Tokens per task, latency to first token, price per successful task |
| Human | Do users accept it? | Thumbs, rubric scores, annotation (Ground Truth), review (A2I) |

### How you run it

| Mechanism | Role |
|---|---|
| Amazon Bedrock model evaluation | Compare models or prompts on your dataset. Automatic metrics and LLM-as-judge, plus human workers |
| LLM-as-a-judge | A stronger model scores outputs with a rubric. Cheap, biased toward its own style. Calibrate with humans |
| Human raters | Ground Truth labeling jobs, or A2I for a live queue. Use for high-stakes and for calibrating the judge |
| A/B or canary | Prompt Management / Flows, or two aliases. Split traffic. Watch quality **and** cost |
| Regression set | Fixed prompts run on every prompt or model change. Fail the pipeline on a drop |
| Quality gate | Deploy only if faithfulness, schema validity, and safety rates hold |
| RAG eval | Score retrieval and generation separately so you do not “fix” chunking by rewriting the prompt |
| Agent eval | Bedrock agent evaluation jobs: did it complete the task and use tools correctly |
| Synthetic plus production sample | Synthetic for coverage of edge cases. Production feedback for what users actually ask |
| Report | QuickSight or Grafana for stakeholders. Include model, prompt version, cost, and quality on one page |

### RAG split (memorize)

If answers are fluent but **wrong**:

1. Context recall low → the right passage was never retrieved (chunking, embedding, query, filters).
2. Context precision low → too much junk in the prompt (k too high, no rerank, no metadata filter).
3. Faithfulness low while context is good → the model ignored the context (prompt, temperature, need grounding guardrail).
4. Answer relevance low while faithful → it quoted the docs but did not answer the question (query rewrite or instructions).

### Agents

Task completion rate is the headline. Also check: wrong tool, right tool with bad arguments, extra steps, skipped confirmation, loop until the step cap. A high completion rate with the wrong tool is a fail if the rubric says which tool is allowed.

### Deploy checks

Before shifting an alias: replay a golden set, compare outputs (semantic diff, not string equality), measure hallucination and schema-fail rates, run the safety set (injection, PII, denied topics). Canary first. Roll the alias back if the gate fails. Synthetic “user journeys” catch multi-turn breaks that single-shot tests miss.

**Pick this when:** “which model should we standardize on” → Bedrock model evaluation with cost in the table. “is the new chunking better” → retrieval recall/precision, not BLEU. “stakeholders want a monthly view” → a report on top of stored eval scores. “only humans can judge tone” → human rubric, judge model optional. “block the deploy” → quality gate in CodePipeline, not a dashboard alone.

## 5.2 Troubleshooting

Work outside in: quota and IAM, then request shape, then prompt, then retrieval, then the model.

### Context window

| Symptom | Cause | Fix |
|---|---|---|
| Truncated middle or error about length | Prompt + history + chunks + tools > window | Fewer chunks, summarize history, hierarchical chunking, larger-context model |
| Answers ignore the start of the prompt | Lost-in-the-middle | Put instructions first and last. Cut noisy chunks |
| Sudden token spike | History appended forever, or a tool result pasted in full | Summarize, cap tool output |
| Partial document | Chunk too small or parser dropped a region | Chunking and parsing, not temperature |

### API integration

| Symptom | Look at |
|---|---|
| `AccessDeniedException` | IAM action, resource ARN (profile **and** foundation model for cross-Region), SCP, model access form (Anthropic use-case), Marketplace subscribe permission, IAM eventual consistency on a new role |
| Malformed / `ValidationException` | InvokeModel body for the wrong provider. Missing field. `maxTokens` too large for the model. On-demand not supported (use inference profile). Prefill removed on newer Claude |
| `ThrottlingException` | Unset `maxTokens` reserving the model max. Real quota. Retry storm. Need cross-Region profile or provisioned throughput |
| Timeout | Output too long, cold start, tool hang. Stream, raise tool timeout, circuit-break |
| Wrong model served | AppConfig stale, alias still on old version, forgot `PrepareAgent` |
| Works in console, fails in VPC | Missing VPC endpoint or endpoint policy |
| Empty tool call | Schema mismatch. Model returned text instead of `toolUse`. Prompt does not require the tool |

Retry: throttling, model timeout, 5xx, service unavailable. Do not retry: validation, access denied, resource not found.

### Prompts

- Version-diff the template that changed.
- Schema validation fails → output contract or a parser retry, not a new embedding model.
- Prompt injection succeeds → guardrail prompt-attack filter and untrusted-content boundaries.
- Inconsistent format → lower temperature, schema, few-shot of the format, stop sequences.
- CloudWatch Logs Insights to group failures by prompt version. X-Ray to see which branch ran.

### Retrieval

| Symptom | Likely cause |
|---|---|
| Irrelevant chunks | No metadata filter, k too high, wrong index, hybrid disabled for keyword-heavy queries |
| No chunks | Filter too strict, ingestion failed, wrong Region, query embedded with a different model |
| Old policy cited | Ingestion lag or deletion policy retained removed files |
| Right doc, wrong paragraph | Chunk size; switch to hierarchical or semantic |
| Table nonsense | Parser split the table. Enable advanced parsing and re-ingest |
| Scores incomparable after a migration | Different distance metric or reranker. Do not mix scores from two stores in one threshold |
| Great in eval, bad in prod | Query distribution shifted. Log real queries and add them to the set |

Embedding drift: you changed models or dimensions and did not re-embed the corpus. Queries and documents must use the same model.

### Agents

- Action group Lambda errors → tool logs, not the foundation model.
- Agent “does nothing new” after an edit → `PrepareAgent` and the alias version.
- Loop → step cap and a stopping condition.
- Return of control with no client handler → the session sits waiting for a tool result you never send.

**Pick this when:** the stem gives a symptom. Match the layer. A faithfulness problem with a bad retriever is not fixed by temperature. A throttle at low QPS is not fixed by a reranker.

## Domain 5 traps

- BLEU/ROUGE as the reason to pick a model for a chatbot.
- One end-to-end “accuracy” number for RAG. Split retrieval from generation.
- LLM-as-judge with no human calibration on a high-stakes workflow.
- Retrying `AccessDeniedException`.
- Re-embedding with a new model into an index built with the old model.
- String-equality tests on generative answers in CI.
- Declaring success on task completion while the agent used a forbidden tool.
