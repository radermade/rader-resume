---
inclusion: auto
name: aws-gen-ai-decisions
description: "Use when designing an AWS generative AI system or choosing among prompting, retrieval, fine-tuning, agents, workflows, models, safety controls, integration styles, or cost levers. Decision order for Amazon Bedrock and related services."
---

# Choose the pattern before the product

Settle the constraint, then the pattern, then the service. A later product does not repair an earlier mismatch.

1. **Constraint.** Who is waiting, how fresh the facts must be, where the data is allowed to live, what must be audited, what a person must approve, and the latency and cost budgets.
2. **Pattern.** Prompt, retrieval, weight customization, agent, or a fixed workflow. The trees below.
3. **Proof.** A Bedrock proof on a labeled set that records quality, latency, and token cost. Build the production shape after that holds.
4. **Shared path.** One front door, versioned prompts, a published guardrail, invocation logs, and an eval gate on the pipeline.

Review the result against the Well-Architected Generative AI Lens: operational excellence, security, reliability, performance, cost, sustainability, and responsible AI.

## Which pattern

```text
Does the task need tools, multi-step decisions, or side effects?
  yes → agent or a workflow (see Agents versus workflows)
  no
    Do answers depend on private or changing facts?
      yes → retrieval
      no
        Is the task a stable format, style, or behavior prompting cannot hold?
          yes → customize weights, then register and deploy
          no → prompt a hosted model
```

- Prices, policies, tickets, and anything edited after deploy belong in retrieval, with citations.
- A voice, a schema, or a domain vocabulary that prompting cannot hold belongs in customization (fine-tune, continued pre-training, or LoRA), then SageMaker Model Registry.
- Tools, branching on intermediate results, and side effects belong in an agent or a state machine.
- One completion with no tool is a prompt. Keep it a prompt.

## Agents versus workflows

```text
Are the steps known in advance?
  yes → Step Functions
        prompt-to-prompt only, edited by people who should not ship code → Prompt Flows
  no, the model must choose tools
    New managed loop, defined as config → AgentCore Harness
    The loop is already written → AgentCore Runtime
    Existing REST APIs should become tools → AgentCore Gateway
    Many specialist bots and a router → Agent Squad, or a supervisor with narrow tools
    An existing classic Bedrock Agent (action groups, aliases) → keep it on that platform
    A person must approve a side effect → Step Functions callback or Augmented AI, in front of the side effect
```

New systems start on AgentCore. Classic Bedrock Agents stay in place when the running system already uses action groups, `PrepareAgent`, and aliases.

Fixed graphs are easier to audit. Prefer them when the path can be drawn before the request arrives.

## Retrieval shape

```text
Need a vector database you operate?
  no → Bedrock managed Knowledge Base
  yes
    Already on Aurora or RDS PostgreSQL → pgvector
    Hybrid search and rich filters, AWS-native → OpenSearch Serverless
    Relationships between entities → Neptune Analytics
    Simplest managed vector bucket → S3 Vectors
    ACL-aware enterprise search, connectors, less control of chunking → Kendra
    Already on Pinecone, Redis, or MongoDB → customer-managed Knowledge Base on that store
```

Chunking, once the data source exists:

- Headings, and the answer needs the section → hierarchical (search the child, return the parent).
- Long prose with topic shifts → semantic.
- FAQs and short uniform articles → fixed size with overlap.
- Tables or figures → advanced parsing, then re-ingest.

Queries that mix error codes and prose → hybrid search. Right documents in the wrong order → rerank. Another tenant’s documents appearing → a metadata filter. A corpus that changes through the day → event-driven incremental ingestion.

## Model and billing

```text
Modality?
  hard text reasoning → a large model
  high volume, simple text → a small model (Haiku, Nova Micro, Nova Lite)
  image → Claude or Nova
  video → a Nova model that lists video
  embeddings → Titan Text Embeddings V2, unless multilingual quality calls for Cohere
  reorder a short retrieved list → a Bedrock reranker

Traffic?
  spiky or unknown → on-demand
  huge, overnight, nobody waiting → batch inference
  flat and high all day, and the math beats on-demand → provisioned throughput
  cannot sit on shared capacity → reserved tier
  throttled, or the model is thin in-region → cross-Region inference profile
  data must stay in a geography → geo prefix (eu. / us. / apac.)

Latency?
  time to first token → streaming, and a smaller model or the Priority tier
  the same prefix on every call → prompt cache
  the same answers every day → precompute
```

Swap models without a code change by keeping the model id in AppConfig (or Parameter Store) behind one gateway.

`global.` inference profiles may leave the geography. A residency requirement uses the geo prefix for that geography.

## Which safety control

```text
Toxic, off-topic, or jailbreak → guardrail content, topic, or prompt-attack filter
PII in the completion → guardrail BLOCK or ANONYMIZE
PII already in a bucket → Macie
PII in text the pipeline submits → Comprehend
Invented facts → retrieval, citations, and contextual grounding
Bad JSON → a schema check
High-stakes side effect → a person
Prove who called → CloudTrail data events and invocation logs
Prove which source → citations, chunk metadata, and the Glue catalog
```

Apply the guardrail on every chat turn with `guardrailConfig`. Skip trusted system text and still scan user text, tool results, and chunks with `guardContent`. Screen a payload with no generation with `ApplyGuardrail`. Streaming compliance uses `streamProcessingMode: sync`.

## How the request moves

```text
User waiting on a short answer → API Gateway, Lambda, Converse
User waiting on tokens → ConverseStream, then WebSocket or response streaming
Work can finish later → SQS or EventBridge
Fan-out and an audit of each step → Step Functions
Callers must not learn Bedrock → a gateway (auth, guardrail, route, log)
On-prem data cannot leave → Outposts, or process in place and send only the allowed prompt
Edge milliseconds, light logic → Wavelength, or Lambda@Edge for request shaping
GPU and weights you trained → a SageMaker endpoint
```

## Which bill to cut

```text
Input tokens on a stable prefix → prompt cache
Input tokens on retrieved text → better retrieval, smaller k
Output tokens → maxTokens and a concise-output instruction
A large model answering easy questions → route those to a small model
Identical calls repeated → an exact result cache
Paraphrased calls repeated → a semantic cache
Idle reserved capacity → move that traffic back to on-demand
Cannot see who spent it → inference profile tags and Cost Explorer
```

A semantic cache is for answers that are safe to share across wordings. Per-user facts stay out of it. Cache after the policy check, and include tenant and policy version in the key.

## Where quality broke

```text
Fluent and wrong, and the context never contained the fact → retrieval (chunk, embed, query, filter)
Context has the fact, and the answer contradicts it → faithfulness (prompt, grounding guardrail)
Faithful, and off the question → query rewrite or instructions
Format breaks the next system → schema, lower temperature
Safety miss → guardrail regression set
Agent finishes by calling the wrong tool → tool-choice metric
Scores dropped after a deploy → alias rollback, diff against the golden set
```

## Throttling

```text
ThrottlingException at a low request rate
  maxTokens unset → set it (unset reserves the model maximum)
  still throttled → compare token metrics with Service Quotas
  need more burst inside the geography → cross-Region profile
  steady traffic above on-demand → provisioned throughput
Retry storm → adaptive retry with jitter
AccessDeniedException → fix IAM; do not retry
```
