---
inclusion: auto
name: aws-gen-ai-retrieval
description: "Use when designing or implementing retrieval-augmented generation on AWS: Knowledge Bases, chunking, embeddings, vector stores, hybrid search, reranking, metadata filters, ingestion, and citations."
---

# Retrieval

Use retrieval when the answer depends on private or changing facts and the user needs a source. The vector store holds embeddings, text, and metadata. It is not a model.

```text
source documents
  → parse (Textract or advanced parsing when there are tables)
  → chunk
  → embed with model M at dimension D
  → upsert into an index whose vector field is D
  → query: optional rewrite → embed with M → filter → kNN (+ keyword if hybrid)
  → optional rerank → prompt → generate → cite
  → on change: incremental ingest, or the index is lying
```

Query embeddings and document embeddings use the same model, the same dimension, and the same normalization. Switching the embedding model means a full rebuild. Mismatched dimensions fail ingestion or retrieve nonsense.

## Knowledge Bases

Managed Knowledge Bases are the default for new retrieval. AWS runs the vector store.

Bring your own store when you already operate it, or when you need control the managed store does not give you: OpenSearch, Aurora or RDS pgvector, Pinecone, Redis, MongoDB Atlas, Neptune Analytics, S3 Vectors.

| Store | Reach for it when |
|---|---|
| Managed Knowledge Base | New retrieval, and you do not want to run a vector database |
| S3 Vectors | The simplest AWS-managed vector bucket |
| OpenSearch Serverless | Hybrid search, filters, scale, no cluster to patch |
| OpenSearch managed | A cluster that already exists; k-NN or the neural plugin next to lexical search |
| Aurora or RDS PostgreSQL + pgvector | The team already runs Postgres. SQL filters plus vectors |
| Neptune Analytics | The useful signal is a relationship (graph retrieval), not only similarity |
| Kendra | Connectors and ACLs, and you want a search product rather than this pipeline |
| DynamoDB | Metadata, keys, and session. DynamoDB is not the ANN index |

Customer-managed OpenSearch Serverless: create the VECTORSEARCH collection and index before the Knowledge Base. Field names match the Knowledge Base config. The data access policy includes the Knowledge Base role.

The Knowledge Base role trusts `bedrock.amazonaws.com` with confused-deputy conditions, reads the corpus bucket, writes the vector store, may call `bedrock:InvokeModel` on the embedding model, uses KMS, and reads Secrets Manager when the store has a password. New roles are eventually consistent. A create that fails once can succeed on retry.

Scope that role to the corpus prefix.

## Chunking

Chunk strategy is locked when the data source is created. Changing it means a new data source, a re-ingest, and a cutover. Old chunks do not repair themselves.

| Strategy | Documents | What goes wrong if you force it |
|---|---|---|
| Fixed size, about 10–20% overlap | FAQs, short uniform docs | Splits sentences, arguments, and tables |
| Semantic | Long docs whose topic drifts | Slower, costlier ingest |
| Hierarchical | Manuals, contracts, anything with headings | More moving parts. Worth it when the answer needs the section |
| None | Already chunked, or shorter than one chunk | You inherit the splits you were given |

Overlap exists so a fact on a boundary is in two chunks. Zero overlap drops boundary facts. Huge chunks waste the window and dilute the embedding. Tiny chunks drop the subject of the paragraph (“it increased 4%”).

Defaults move. Remember the relationships, not a magic token count.

Turn on advanced parsing for tables, multi-column layouts, and figures, then re-ingest. Default text splitting tears tables apart. Scrambled numbers from a table are a parser problem.

## Metadata

Filter before similarity. A prompt instruction to “only use tenant acme” arrives after the other tenants’ chunks are already in the context.

Bedrock reads a sidecar beside the object. For `policy.pdf` the file is `policy.pdf.metadata.json`:

```json
{
  "metadataAttributes": {
    "tenant": "acme",
    "department": "hr",
    "year": 2026,
    "classification": "internal"
  }
}
```

- Tenant, ACL, language, document type, and effective date are filterable fields, mapped as keywords or numbers.
- The same keys exist on every object. A missing key makes the filter drop the document silently.
- S3 object metadata is fine for simple tags. The sidecar is what Knowledge Base ingestion stores as attributes.
- Separate indexes when corpora differ in embedding model, access policy, language, or update rate.
- `Retrieve` filters use the Knowledge Base filter language (`equals`, `in`, `andAll`, `orAll`), including when the store is OpenSearch.

## Search

| Mode | When |
|---|---|
| Semantic | Paraphrase and conceptual questions |
| Keyword / BM25 | SKUs, error codes, exact names, rare tokens |
| Hybrid | Both. Product ids inside prose is the usual case |
| Rerank | Recall is acceptable and the order is wrong. A second model reorders a short list (for example 50 down to 5) |

`overrideSearchType` of `HYBRID` or `SEMANTIC` works on OpenSearch Serverless, Aurora, RDS, and MongoDB Atlas when a filterable text field exists. Other stores are semantic. Omit the override and Bedrock chooses. Setting hybrid on a store that cannot do it fails validation.

Rerank is not a substitute for chunking. Larger `k` raises recall and hurts precision, cost, and faithfulness. After a rerank, stuff a smaller `k`.

| Query problem | Fix |
|---|---|
| Vague question | Rewrite or expand before search |
| Multi-hop (“compare A and B given policy C”) | Decompose, retrieve each, merge |
| Domain jargon, or the query looks nothing like the docs | Expand acronyms, or HyDE (embed a hypothetical answer and search with that vector) |
| Near-duplicate chunks | Maximal marginal relevance or a dedupe step |
| The app needs a stable way to search | A retrieve API or a tool. One implementation, called by the model |

## Freshness

Stale vectors are a common cause of confident wrong answers.

| Corpus | Pattern |
|---|---|
| Small, changes rarely | EventBridge schedule, full ingestion |
| Objects landing in S3 through the day | Event to `StartIngestionJob` or incremental sync |
| OLTP rows | DynamoDB Streams, Kinesis, or EventBridge → embed → upsert |
| Deletes | A deletion policy that removes vectors. Retaining deleted documents keeps them citable |

Monitor lag: newest source timestamp versus newest indexed timestamp. Reindex after an embedding-model change.

## APIs

| Call | Client | Returns |
|---|---|---|
| `CreateKnowledgeBase`, `CreateDataSource`, `StartIngestionJob` | `bedrock-agent` | Control plane. Ingestion is async. Poll the job |
| `Retrieve` | `bedrock-agent-runtime` | Chunks, scores, metadata. You write the prompt |
| `RetrieveAndGenerate` | `bedrock-agent-runtime` | Answer, citations, `sessionId`. Pass `sessionId` on the next turn |
| `AgenticRetrieveStream` | SDK, managed Knowledge Base | Multi-step retrieve |

Guardrails on `RetrieveAndGenerate` are opt-in. Pass `guardrailConfiguration` with id and version.

`Retrieve` and `RetrieveAndGenerate` are CloudTrail data events. A trail of management events misses them.

## When the answer is wrong

| What you see | Fix |
|---|---|
| Right document, wrong slice | Chunking, or hierarchical parent |
| The document is never retrieved | Ingestion, filter, embedding mismatch, wrong index |
| It comes back at rank 30 | Hybrid, rerank, or a better embedding fit |
| Last year’s PDF is cited | Freshness, or a metadata date filter |
| Another tenant’s text | Filter, an index per tenant, or IAM on the source |
| Table numbers are scrambled | Parser, then re-ingest |
| A good chunk is ignored | Prompt, lost-in-the-middle, `k` too high, grounding |
| Ingestion fails on dimension | Index mapping and the embedding model disagree |
| Scores from two stores disagree | Different distance metrics. Keep thresholds per store |

Retrieved text is untrusted. Indirect prompt injection hides in a PDF. Run the prompt-attack filter on chunks. See `safety.md`.
