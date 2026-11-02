# Glossary

Short definitions in the sense this exam uses them.

| Term | Meaning |
|---|---|
| Foundation model (FM) | Pretrained model you prompt or adapt. You do not train it from scratch on this exam |
| Converse API | Unified Bedrock inference API across providers |
| InvokeModel | Provider-specific inference body |
| Inference profile | Indirection to a model, often cross-Region (`us.`, `eu.`, `apac.`, `global.`) or an application profile you can tag |
| Cross-Region inference | Bedrock routes the request inside a geography (or globally) for capacity |
| Provisioned throughput | Reserved Bedrock capacity, model units, commitment. Required for many custom models |
| On-demand | Pay per token, shared capacity |
| Batch inference | Async discounted inference for bulk jobs |
| Service tier | Priority, Standard, or Flex on on-demand. Latency vs price |
| `maxTokens` | Output cap. Also drives how much quota a request reserves |
| Temperature | Randomness. Low = stable. Not a truth setting |
| top-p / top-k | Nucleus / truncated sampling. Diversity controls |
| Stop sequence | String that ends generation |
| Context window | Max tokens of prompt + completion the model accepts |
| Lost in the middle | Models use the start and end of a long prompt better than the middle |
| Token | Subword unit. Billing and limits are in tokens, not characters |
| Prompt caching | Reuse of an identical prefix. `cachePoint` marker. Exact match |
| Semantic cache | Cache keyed by similarity of the question, in your app or ElastiCache |
| Embedding | Vector that places text (or images) in a similarity space |
| Dimension | Length of that vector. Must match the index |
| Chunk | Piece of a document you embed and retrieve |
| Overlap | Tokens shared by neighboring chunks so boundary facts survive |
| Hierarchical chunking | Small child chunks for search, parent chunk for context |
| Semantic chunking | Split where the topic shifts |
| Advanced parsing | FM or layout parser so tables and figures stay intact |
| Vector store | Index for nearest-neighbor search over embeddings |
| k-NN / ANN | Nearest neighbor search, exact or approximate |
| Hybrid search | Keyword score plus vector score |
| Reranker | Second model that reorders a short candidate list |
| Metadata filter | Structured predicate applied with the search (tenant, date, type) |
| RAG | Retrieve chunks, put them in the prompt, generate, ideally with citations |
| Faithfulness | Answer stays inside the retrieved context |
| Grounding | Same idea, often the guardrail check against sources |
| Hallucination | Confident text that is not supported |
| Knowledge Base | Bedrock managed RAG resource: data sources, chunking, embeddings, store |
| Managed KB | Bedrock runs the vector store |
| Customer-managed KB | You supply OpenSearch, Aurora, Pinecone, and so on |
| Ingestion job | Async process that chunks, embeds, and upserts |
| `Retrieve` | Returns chunks. You call the model |
| `RetrieveAndGenerate` | Returns an answer and citations |
| Session id | Conversation state for multi-turn RetrieveAndGenerate |
| Sidecar metadata | `file.metadata.json` next to the object |
| Agent | Model loop that can call tools and stop |
| Action group | Classic agent tool: OpenAPI schema + Lambda or return of control |
| Return of control | Agent pauses and your code runs the tool |
| Agent alias | Stable endpoint that points at a prepared agent version |
| PrepareAgent | Builds a snapshot after config changes. Required |
| AgentCore | Newer Bedrock agent platform: Runtime, Gateway, Memory, Harness, evaluations |
| Harness | Managed agent loop defined as configuration |
| Runtime | AgentCore host for a loop you wrote |
| Gateway | AgentCore front that exposes APIs as MCP tools |
| Strands Agents | SDK for agents and tools |
| Agent Squad | Multi-agent router to specialist agents |
| MCP | Protocol for discovering and calling tools |
| ReAct | Reason, act (tool), observe, repeat |
| Chain-of-thought | Model writes intermediate steps, no tool required |
| Prompt chain | Fixed sequence of prompts |
| Prompt Flows | Bedrock visual designer for chains and branches |
| Prompt Management | Versioned, parameterized prompt templates |
| Tool use | Converse `toolUse` / `toolResult` blocks. Your code runs the tool |
| Guardrail | Bedrock policy for content, topics, words, PII, grounding |
| Denied topic | Custom subject the guardrail blocks |
| Prompt injection | User text that tries to override instructions |
| Indirect injection | Malicious instructions inside retrieved or tool data |
| `BLOCK` vs `ANONYMIZE` | Reject the call vs redact the span |
| Contextual grounding threshold | Min support from sources |
| Relevance threshold | Min on-topic score vs the user question |
| ApplyGuardrail | Evaluate text without invoking a model |
| Model card | Document of intended use, limits, and data |
| Responsible AI | Fairness, privacy, robustness, transparency, governance, controllability |
| LLM-as-judge | A model scores outputs with a rubric |
| Golden set | Fixed eval prompts and expected traits |
| Canary | Small traffic slice on the new version |
| Quality gate | Pipeline check that blocks a deploy |
| A2I | Augmented AI. Human review of model output |
| Ground Truth | Labeling workforce and jobs for datasets |
| Clarify | SageMaker bias and explainability for traditional models |
| Model Registry | Version, approve, and deploy SageMaker models |
| LoRA | Low-rank adapters. Parameter-efficient fine-tune |
| Continued pre-training | More unsupervised training on domain text |
| Fine-tuning | Supervised adaptation on input/output pairs |
| Circuit breaker | Stop calling a failing dependency, then probe again |
| Graceful degradation | Smaller model, cache, or non-AI fallback |
| GenAI gateway | Shared front door: auth, routing, guardrails, logs |
| Inference invocation logs | Stored prompts and completions. Sensitive |
| Data event | CloudTrail event for data-plane calls. Off by default |
| PrivateLink / interface endpoint | Private VPC path to Bedrock |
| Confused deputy | A service role misused across accounts. Condition `aws:SourceArn` / `aws:SourceAccount` |
| Well-Architected Generative AI Lens | AWS review checklist for genAI workloads |
| Q Developer | Assistant for builders in the IDE and CLI |
| Q Business | Assistant for employees over enterprise content |
| Kendra | Managed enterprise search |
| Textract | OCR and forms |
| Comprehend | NLP: entities, sentiment, PII on text you send |
| Macie | PII and sensitive data discovery in S3 |
| Transcribe | Speech to text |
| Lex | Conversational bots with intents and slots |
| Data Automation | Bedrock feature for processing documents and media into structured output |
| Outposts | AWS racks in your data center |
| Wavelength | AWS compute in a telecom edge |
| AppConfig | Hosted configuration, feature flags, safe deployment of config |
| Compensatory scoring | Pass the whole exam. No per-domain cut score |
| Unscored item | 10 of 75 questions do not count and are not marked |
