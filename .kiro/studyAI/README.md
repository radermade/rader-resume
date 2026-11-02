# AIP-C01 study base

Cheat sheets for the **AWS Certified Generative AI Developer – Professional (AIP-C01)** exam. Product facts and decision rules, written to answer scenario questions. Not a course.

Official outline: [AIP-C01 exam guide](https://docs.aws.amazon.com/aws-certification/latest/ai-professional-01/ai-professional-01.html) (2026). Bedrock APIs move. Treat model IDs, quotas, and default chunk sizes as “verify before production,” and treat the *choice* (which service, which pattern) as what the exam scores.

## Exam facts

| Item | Value |
|---|---|
| Questions | 75 (65 scored, 10 unscored and unmarked) |
| Time | 180 minutes |
| Types | Multiple choice (1 of 4) and multiple response (2+ of 5+; all must be selected) |
| Score | Scaled 100–1000. Pass **750**. Compensatory: no per-domain pass line |
| Guessing | Unanswered = wrong. No penalty for a wrong guess |
| Candidate | 2+ years production apps, general AI/ML or data, 1 year hands-on GenAI |
| Out of scope | Training models from scratch, advanced ML technique, feature engineering |

## Weights

| Domain | Weight | File |
|---|---|---|
| 1. Foundation model integration, data management, compliance | 31% | [domains/01-foundation-model-integration.md](domains/01-foundation-model-integration.md) |
| 2. Implementation and integration | 26% | [domains/02-implementation-and-integration.md](domains/02-implementation-and-integration.md) |
| 3. AI safety, security, and governance | 20% | [domains/03-ai-safety-security-governance.md](domains/03-ai-safety-security-governance.md) |
| 4. Operational efficiency and optimization | 12% | [domains/04-operational-efficiency.md](domains/04-operational-efficiency.md) |
| 5. Testing, validation, and troubleshooting | 11% | [domains/05-testing-validation-troubleshooting.md](domains/05-testing-validation-troubleshooting.md) |

Domains 1–3 are 77% of the score. Spend study time there first.

## How to use this

1. Read the domain file for the weight you are weakest on. Each task ends with **pick this when**.
2. Drill [review/flashcards.md](review/flashcards.md). Cover the answer line.
3. The night before, read only [review/quick-review.md](review/quick-review.md) and [reference/exam-traps.md](reference/exam-traps.md).
4. When a question names an API, metric, or service, jump to `reference/`.

## Reference

| File | Use it for |
|---|---|
| [bedrock-apis.md](reference/bedrock-apis.md) | Endpoints, Converse vs InvokeModel, KB and agent calls |
| [decision-trees.md](reference/decision-trees.md) | RAG vs fine-tune vs agent, model, vector store, throughput |
| [evaluation-metrics.md](reference/evaluation-metrics.md) | Generation, retrieval, RAG, agent metrics |
| [exam-traps.md](reference/exam-traps.md) | Distractors that sound right |
| [guardrails.md](reference/guardrails.md) | Filters, three apply modes, PII, grounding |
| [glossary.md](reference/glossary.md) | Terms the questions assume |
| [service-cheatsheets.md](reference/service-cheatsheets.md) | One-liners: when this service wins |
| [rag-and-retrieval.md](reference/rag-and-retrieval.md) | Chunking, embeddings, hybrid search, metadata |
| [agents-mcp-patterns.md](reference/agents-mcp-patterns.md) | Agents, AgentCore, Strands, MCP, ReAct |
| [troubleshooting-errors.md](reference/troubleshooting-errors.md) | Error → cause → fix |
| [in-scope-and-out-of-scope.md](reference/in-scope-and-out-of-scope.md) | Services the guide allows and forbids |

## Product vs exam wording

The 2026 guide still tests **classic Bedrock Agents** (action groups, aliases, `PrepareAgent`), **Strands Agents**, **AWS Agent Squad**, and **Prompt Flows**. Classic Agents are in maintenance for new customers; new builds use **Bedrock AgentCore**. Learn both. If the question says “action group” or “agent alias,” answer with classic Agents. If it says “host your own agent loop” or “expose REST APIs as MCP tools,” answer with AgentCore Runtime or Gateway.

Managed Knowledge Bases are the current default (AWS owns the vector store). Customer-managed KBs (you bring OpenSearch, Aurora, Pinecone, and so on) are what most exam scenarios still describe, because they force a vector-store choice.
