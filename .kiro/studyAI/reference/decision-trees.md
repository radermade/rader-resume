# Decision trees

Use the stem’s constraint as the first branch, not your favorite service.

## 1. Which pattern?

```text
Does the task need tools, multi-step decisions, or side effects?
  yes → agent or Step Functions (tree 2)
  no
    Do answers depend on private or changing facts?
      yes → RAG (tree 3)
      no
        Is the task a stable format/style/behavior prompting cannot hold?
          yes → fine-tune / LoRA, then registry + deploy
          no → prompt a hosted model (tree 4)
```

Never fine-tune to store this quarter’s prices. Never RAG a pure style change (“always write in our voice”) unless the voice guide is a document you want cited.

## 2. Agent vs workflow

```text
Are the steps known in advance?
  yes → Step Functions (or Prompt Flows if it is only prompt-to-prompt)
  no, the model must choose tools
    Existing classic agent with action groups? → Bedrock Agents
    New managed loop, config not code? → AgentCore Harness
    You already wrote the loop? → AgentCore Runtime
    Many specialist bots and a router? → Agent Squad or a supervisor agent
    Existing REST APIs as tools? → AgentCore Gateway (MCP)
    A person must approve? → Step Functions callback or A2I, in front of the side effect
```

## 3. RAG design

```text
Need a vector database you control?
  no → Bedrock managed Knowledge Base
  yes
    Already on Aurora/RDS Postgres? → pgvector
    Need hybrid + rich filters, AWS-native? → OpenSearch Serverless
    Graph relationships? → Neptune Analytics
    Simplest bucket of vectors? → S3 Vectors
    ACL-aware enterprise search product? → Kendra
    Already on Pinecone / Redis / Mongo? → customer-managed KB on that store

Documents have headings and you need the section, not a fragment?
  → hierarchical chunking
Long prose, topic shifts?
  → semantic chunking
FAQs and short articles?
  → fixed size + overlap
Tables or figures?
  → advanced parsing, then re-ingest

Queries mix error codes and prose?
  → hybrid search
Right docs, wrong order?
  → rerank
Wrong tenant’s docs appear?
  → metadata filter, not a prompt instruction
Corpus changes daily?
  → event-driven incremental ingestion
```

## 4. Model and billing

```text
Modality?
  text only, hard reasoning → large model
  text, high volume, simple → small model (Haiku, Nova Micro/Lite)
  image → Claude or Nova
  video → Nova class that lists video
  embed → Titan Embeddings V2 unless multilingual quality says Cohere
  rerank → Bedrock reranker

Traffic?
  spiky or unknown → on-demand
  huge overnight, no human waiting → batch
  flat and high all day → provisioned throughput (only if the math beats on-demand)
  cannot tolerate shared capacity → reserved tier
  throttled or model thin in-region → cross-region inference profile
  data must stay in-geography → geo prefix (eu. / us. / apac.), never global.

Latency?
  time-to-first-token → streaming + smaller or priority tier
  same prompt prefix every call → prompt cache
  same answers every day → precompute
```

Swap models without redeploying code → model id in AppConfig, one gateway.

## 5. Safety control

```text
What is the failure?
  Toxic or off-topic or jailbreak → guardrail content / topic / prompt-attack
  PII in the completion → guardrail BLOCK or ANONYMIZE
  PII sitting in a bucket → Macie
  PII in a text pipeline you run → Comprehend
  Invented facts → RAG + citations + contextual grounding
  Bad JSON → schema check, not a content filter
  High-stakes side effect → human review
  Must prove who called → CloudTrail data events + invocation logs
  Must prove which source → citations + metadata + Glue catalog
```

Apply mode:

```text
Every turn, input and output → guardrailConfig
Skip trusted system text, still scan user/tool/chunks → guardContent
Screen a payload with no generation → ApplyGuardrail
Streaming + compliance → streamProcessingMode sync
```

## 6. Integration style

```text
User waiting on a short answer? → API Gateway + Lambda + Converse
User waiting on tokens? → ConverseStream + WebSocket or response streaming
Work can finish later? → SQS or EventBridge
Fan-out and audit of steps? → Step Functions
Legacy must not learn Bedrock? → gateway (auth, guardrail, route, log)
On-prem data cannot leave? → Outposts or process in place
Edge milliseconds, light logic? → Wavelength or Lambda@Edge
Heavy GPU you trained? → SageMaker endpoint, not Lambda
```

## 7. Cost lever

```text
Bill is input tokens on a stable prefix? → prompt cache
Bill is input tokens on retrieved text? → better retrieval, smaller k
Bill is output tokens? → maxTokens and “be concise”
Bill is the large model on easy questions? → route to a small model
Bill is repeated identical calls? → exact cache
Bill is repeated paraphrases? → semantic cache (not for per-user facts)
Bill is idle reserved capacity? → move back to on-demand
Cannot see who spent it? → inference profile tags + Cost Explorer
```

## 8. Eval failure

```text
Fluent and wrong, context missing the fact → retrieval (chunk, embed, query, filter)
Context has the fact, answer contradicts it → faithfulness (prompt, grounding guardrail)
Answer is faithful but off-topic → query rewrite or instructions
Format breaks downstream → schema + lower temperature
Safety miss → guardrail regression set
Agent “succeeds” via the wrong tool → tool-choice metric, not completion alone
Scores dropped after deploy → alias rollback, diff against golden set
```

## 9. Throttling

```text
Low request rate but ThrottlingException?
  maxTokens unset? → set it
  still throttled? → Service Quotas vs token metrics
  need burst in-geo? → cross-region profile
  steady and above on-demand? → provisioned
Retry storm? → adaptive retry + jitter, not a tight loop
AccessDenied? → do not retry; fix IAM
```
