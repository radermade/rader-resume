# Domain 1 — Foundation model integration, data management, and compliance

**31% of scored content.** Tasks 1.1–1.6. This domain is architecture choice: which model, which data path, which retrieval design, which prompt control. Implementation mechanics live in Domain 2. Safety controls live in Domain 3.

## 1.1 Design the solution before picking a model

Match the constraint in the stem, in this order:

1. **Business constraint** — latency, cost, residency, freshness of facts, audit, human approval.
2. **Pattern** — prompt only, RAG, fine-tune, agent, or a cascade. See [decision-trees.md](../reference/decision-trees.md).
3. **Proof** — a Bedrock PoC that measures quality, latency, and token cost before a full build.
4. **Standard component** — Well-Architected Generative AI Lens: operational excellence, security, reliability, performance, cost, sustainability, plus responsible AI.

**Pick this when**

| Stem says | Answer leans |
|---|---|
| Facts change, must cite internal docs | RAG / Knowledge Base, not fine-tuning |
| Stable style, format, or tone that prompting cannot hold | Fine-tune or LoRA, then Model Registry |
| Multi-step, tools, branching decisions | Agent or Step Functions orchestration |
| One-shot classification or extraction | Small model, low temperature, structured output |
| Must prove value before build | Bedrock PoC with an eval set, not a production agent |
| “Consistent across teams” | Shared prompt templates, guardrail, gateway, Well-Architected GenAI Lens |

Do not fine-tune to memorize a policy PDF. Do not build an agent for a single completion.

## 1.2 Select and configure foundation models

### Selection

| Need | Lean toward |
|---|---|
| Hard reasoning, long instructions | Larger Claude (Opus / Sonnet class) |
| Cheap, fast, high volume | Haiku or Nova Micro / Lite |
| Text + image | Claude (image) or Nova (image, and video on Nova Pro/Lite class) |
| Open weights, Bedrock fine-tune | Llama (and other models Bedrock currently allows to customize) |
| Embeddings | Titan Text Embeddings V2 (default, dimensions configurable) or Cohere Embed |
| Rerank a retrieved list | Bedrock reranker (Cohere Rerank or Amazon Rerank — use what the question names) |

Check capabilities with `list-foundation-models`. Do not memorize model ID strings. Memorize the **tradeoff**: quality vs cost vs latency vs modality vs customization vs context window.

Some models reject on-demand base IDs. `ValidationException: on-demand throughput isn't supported` means switch to an **inference profile** ID (`us.`, `eu.`, `apac.`, `global.`) or a provisioned-throughput ARN.

### Model ID shapes

| Access | Shape |
|---|---|
| On-demand, one Region | `provider.model-name` |
| Cross-Region inference | `geo.provider.model-name` (`us.`, `eu.`, `apac.`, `global.`) |
| Application inference profile | `arn:...:inference-profile/id` |
| Provisioned throughput | `arn:...:provisioned-model/id` |

`us.` / `eu.` / `apac.` stay inside that geography (residency). `global.` may leave the geography. If the stem says “data must stay in the EU,” `global.` is wrong and `eu.` is right.

Cross-Region IAM is a classic miss: allow **both** the inference-profile ARN and the foundation-model ARN with a **wildcard Region** (`arn:aws:bedrock:*::foundation-model/...`). A single-Region model ARN denies the routed Region.

### Swap models without a code change

Put the model ID in **AppConfig** (or Parameter Store / a config table) and resolve it in Lambda behind API Gateway. The app calls one interface. Routing logic is config.

Static routing: code or API Gateway maps a route to a model. Dynamic routing: Step Functions (or a gateway) picks the model from content, complexity, or live metrics. Cascade: cheap model first, escalate to a larger model when confidence is low or the task is hard.

### Resilience

- **Cross-Region inference** when a model is thin in one Region or you need more throughput.
- **Step Functions circuit breaker**: count failures, open the circuit, stop calling, fail over or degrade.
- **Graceful degradation**: smaller model, cached answer, “try later,” or a non-AI path. Do not retry `ValidationException` or `AccessDeniedException`.
- **Rollback**: Model Registry version + alias. Keep the last good endpoint. Automate promote and revert.

### Customization lifecycle (deploy, not “how to train”)

Training from scratch is **out of scope**. The exam still asks how you **ship** a customized model:

| Technique | What it changes | Use when |
|---|---|---|
| Prompting | Nothing in weights | First choice |
| RAG | Retrieved context | Facts must be current and cited |
| Fine-tuning | Behavior, format, style | Labeled input/output pairs, stable task |
| Continued pre-training | Domain language in the weights | Unlabeled domain corpus, new vocabulary |
| LoRA / adapters | Small trainable matrices on a frozen model | Domain adaptation with less compute; often SageMaker |
| Full fine-tune | More weights | Last resort; cost and catastrophic forgetting |

Lifecycle: train or adapt → register in **SageMaker Model Registry** → approval → deploy to a SageMaker endpoint or Bedrock custom-model throughput → canary → rollback → retire old versions. Bedrock custom models often require **provisioned throughput** to invoke. On-demand is for serverless base models.

**Pick this when:** “no code change to swap providers” → AppConfig + gateway. “model missing in this Region” → cross-Region inference profile, not a second hand-rolled endpoint, unless they need a model Bedrock does not host.

## 1.3 Pipelines that feed the model

The model is only as good as the input shape.

| Job | Service |
|---|---|
| Quality rules on tables (completeness, range, freshness) | Glue Data Quality |
| Visual prep of tabular ML data | SageMaker Data Wrangler |
| Row-level custom checks | Lambda |
| Alert on bad batches | CloudWatch metrics + alarm |
| OCR / forms | Textract |
| Speech to text | Transcribe |
| Entities, sentiment, PII on text | Comprehend |
| Images (labels, moderation) | Rekognition |
| Heavy batch transform | SageMaker Processing |
| Multimodal understanding | Bedrock multimodal model (do not build a Transcribe+Textract pipeline if one model call is the requirement) |

Then format for the API:

- Bedrock Converse: `messages[]` of `user` / `assistant`, optional top-level `system`, `inferenceConfig`.
- InvokeModel: **provider-specific JSON**. Titan body on a Claude model → `ValidationException` / malformed input.
- Dialog: alternate roles. Two user turns in a row is invalid on many models.
- SageMaker endpoint: whatever the container contract is (often CSV/JSON lines). Do not send a Converse body to a SageMaker endpoint.

**Improve inputs before inference:** Comprehend to pull entities, Lambda to normalize units and dates, a cheap Bedrock call to rewrite messy text into the schema the main prompt expects. Garbage in is the usual cause of “the model is dumb.”

**Pick this when:** the stem is about *data quality before* the model, not about the answer quality after. Glue Data Quality and Wrangler are not evaluators of hallucinations.

## 1.4 Vector store design

A vector store holds embeddings plus text plus metadata so retrieval can augment the prompt. It is not a model.

| Store | Reach for it when |
|---|---|
| Bedrock Knowledge Base, managed | New RAG, you do not want to run a vector DB |
| Bedrock KB, customer-managed | The stem already has OpenSearch, Aurora, Pinecone, Redis, MongoDB, Neptune Analytics, or S3 Vectors |
| OpenSearch Serverless (vector) | Hybrid search, filters, scale, Bedrock integration, no cluster to patch |
| OpenSearch managed + neural plugin | Existing cluster; k-NN / neural search next to lexical search |
| Aurora or RDS + pgvector | Team already runs Postgres; SQL filters + vectors; cost-sensitive |
| DynamoDB **plus** a vector engine | DynamoDB holds metadata and keys; it is not the ANN index by itself |
| Neptune Analytics | Relationships matter (graph RAG), not just similarity |
| S3 Vectors | Simplest AWS-managed vector bucket; Bedrock can create it |
| Kendra | ACL-aware enterprise search and connectors; less control over chunking and embeddings |

### Metadata

Filter **before** similarity, or you search the whole corpus and leak other tenants’ chunks.

- S3 user metadata and sidecar files for source, author, timestamp, department, classification.
- Bedrock KB: `document.pdf.metadata.json` beside the object, body `metadataAttributes`.
- OpenSearch: map filter fields as keyword / numeric, not only as text.
- Tag domain so you can use **one index per domain** or a metadata filter. Multi-index when corpora differ in embedding model, access policy, or update rate.

### Scale

- Shard or partition hot indices. Do not one-shard a global corpus.
- Separate indices per language, tenant, or domain when they do not belong in one similarity space.
- Hierarchical index: search child chunks, return the parent section (see retrieval).

### Keep it fresh

Stale vectors are a top hallucination cause.

| Pattern | Mechanism |
|---|---|
| Scheduled full refresh | EventBridge schedule → ingestion job. Use for small, slow corpora |
| Incremental | S3 event or crawler → ingest only new/changed objects |
| Near real time | DynamoDB Streams / Kinesis / EventBridge → embed → upsert |
| Deletes | Tombstone or deletion policy. Retaining deleted docs makes the model cite them |

Ingestion embedding uses the **same** model as query embedding. Mismatched dimensions fail ingestion or silently retrieve nonsense.

**Pick this when:** “semantic search at scale with filters” → OpenSearch or a Bedrock KB on OpenSearch. “already on Aurora” → pgvector. “org chart / citations between entities” → Neptune. “don’t operate a database” → managed KB or S3 Vectors.

## 1.5 Retrieval for augmentation

### Chunking (locked when the data source is created)

Changing strategy means delete and recreate the data source, then re-ingest.

| Strategy | Documents | Cost |
|---|---|---|
| Fixed size, overlap ~10–20% | FAQs, short uniform docs | Splits sentences and tables |
| Semantic | Long technical docs, reports | Slower ingest, chunks follow topic shifts |
| Hierarchical | Manuals, legal, anything with headings | Retrieve small child, expand to parent. Best quality, heaviest |
| None | Already chunked, or doc under ~300 tokens | You own the splits |

Tables and figures: turn on **advanced / FM parsing**. Default text splitting tears tables apart.

Overlap exists so a fact on a boundary is in two chunks. Zero overlap drops boundary facts. Huge chunks waste the context window and dilute relevance. Tiny chunks lose the subject of the paragraph.

### Embeddings

- One model per index. Titan Text Embeddings V2 is the usual default (1024 dims, can reduce). Cohere Embed when multilingual quality is the requirement.
- Batch embeds with Lambda or the KB ingestion job. Do not call the model once per token.
- Query and document embeddings must match model **and** dimension **and** normalization.
- Multimodal embeddings only if you retrieve images by meaning. Do not embed images with a text-only model.

### Search modes

| Mode | When |
|---|---|
| Semantic only | Paraphrase, conceptual questions |
| Keyword / BM25 | SKUs, error codes, exact names, rare tokens |
| Hybrid | Both. Product IDs inside prose is the usual case |
| Rerank | Recall is fine, top-k order is not. Second model reorders a short list |

Bedrock `overrideSearchType`: `HYBRID` or `SEMANTIC` on OpenSearch Serverless, Aurora/RDS, and MongoDB Atlas when a filterable text field exists. Other stores: semantic only. Omit the override and Bedrock picks.

### Query handling

| Problem | Fix |
|---|---|
| User is vague | Rewrite or expand the query (Bedrock or a small model) before search |
| Multi-hop (“compare A and B given policy C”) | Decompose into subqueries (Lambda or Step Functions), retrieve each, merge |
| Domain jargon | Expand acronyms, or HyDE: embed a hypothetical answer and search with that vector |
| Too many near-duplicates | Maximal marginal relevance or a dedupe step before the prompt |
| Access path for the app | Tool / function call the model can invoke, an MCP tool, or a stable retrieve API. Do not paste a new SDK call into every prompt |

**Pick this when:** “exact error code plus a description” → hybrid. “top results feel random” → rerank or metadata filter, not a bigger LLM. “answers miss the section heading” → hierarchical chunking. “numbers from a table are wrong” → parsing, not temperature.

## 1.6 Prompt engineering and governance

### Instruction stack

1. **System / role** — who the model is, what it must refuse, tone, tool rules.
2. **Context** — retrieved chunks, each labeled with a source id.
3. **User task** — the question.
4. **Output contract** — JSON schema, headings, citation format, max length.
5. **Reasoning** — chain-of-thought when the task is multi-step and you can afford tokens. For latency or leakage risk, ask for the answer only, or reason in a scratchpad you strip.

Bedrock **Prompt Management**: parameterized templates, variables, versions, and an approval path. Store the repo in S3 if they want files. Log reads and publishes with CloudTrail and CloudWatch Logs.

Bedrock **Guardrails** enforce safety policy. They are not a substitute for a good system prompt, and a system prompt is not a substitute for a guardrail. See [guardrails.md](../reference/guardrails.md).

### Conversation state

| Need | Store |
|---|---|
| Intent of this utterance | Comprehend or the model itself |
| Clarifying question loop | Step Functions (wait, branch) |
| History | DynamoDB. Summarize old turns. Do not resend the entire transcript every call |
| Session for RetrieveAndGenerate | Pass `sessionId` or each turn is stateless |

### Quality of prompts

- Version prompts. Diff them like code.
- Regression set in CloudWatch or a Step Functions test flow: happy path, edge, injection, empty retrieval.
- Lambda asserts schema and required fields before the response reaches the user.
- Iterate with an eval set (Domain 5), not by vibes.

### Patterns the exam names

| Pattern | Shape |
|---|---|
| Zero-shot | Instruction only |
| Few-shot | 2–5 examples in the prompt (or cached) |
| Chain-of-thought | “Think step by step,” or structured steps |
| Prompt chain | Output of prompt A is input of prompt B |
| Prompt Flows | Visual chain, branches on model output, reusable nodes, pre/post steps |
| ReAct | Thought → action → observation, loop. Often Step Functions or an agent |
| Structured output | JSON schema so the next system can parse |

**Pick this when:** “same prompt, many apps, change control” → Prompt Management versions, not a string in source. “branch on the model’s answer” → Prompt Flows or Step Functions. “keep a long system prompt cheap” → prompt caching (Domain 4), not a shorter policy.

## Domain 1 traps

- Choosing SageMaker training when the stem says the corpus changes weekly. That is RAG.
- Choosing Kendra and OpenSearch as if they were the same. Kendra is managed enterprise search. OpenSearch is the engine you tune.
- Putting tenant id only in the prompt. Put it in a **metadata filter**.
- Hybrid search on a store that only supports semantic.
- Changing chunk strategy in place. You cannot.
- `global.` inference profile when the stem requires Regional residency.
- Glue Data Quality as a hallucination metric.
