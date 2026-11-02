# RAG and retrieval

## Pipeline

```text
source documents
  → parse (Textract / advanced parsing if tables)
  → chunk
  → embed with model M at dimension D
  → upsert into an index whose vector field is D
  → on query: optional rewrite → embed with M → filter → kNN (+ keyword if hybrid)
  → optional rerank → stuff prompt → generate → cite
  → on change: incremental ingest or the index lies
```

Query embedding and document embedding are the same model and the same dimension. Switching M means a full rebuild.

## Chunking

Locked at data-source creation. To change it: new data source, re-ingest, cut over.

| Strategy | Use | Failure if misused |
|---|---|---|
| Fixed size, 10–20% overlap | Short, uniform, FAQs | Splits tables and arguments |
| Semantic | Long docs with drifting topics | Slower and costlier ingest |
| Hierarchical | Manuals, contracts, anything with headings | More moving parts. Worth it when the answer needs the section |
| None | Pre-chunked, or already smaller than one chunk | You inherit bad splits |

Defaults move. Remember the relationships, not a magic token count. Overlap saves boundary facts. Chunks larger than the useful fact waste context and dilute the embedding. Chunks smaller than the fact drop the subject (“it increased 4%”).

Advanced parsing before chunking when the source has tables, multi-column layouts, or figures. Re-ingest after you enable it. Old chunks do not repair themselves.

## Metadata

Filter at retrieval time.

Bedrock KB sidecar, next to `policy.pdf`:

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

The file name is `policy.pdf.metadata.json`.

Design rules:

- Tenant, ACL, language, doc type, and effective date should be filterable fields, mapped as keywords or numbers.
- Put the same keys on every object or the filter silently drops them.
- Do not rely on the model to “only use tenant acme” after you retrieved everyone.
- S3 object metadata is useful for simple tags. The sidecar is what KB ingestion understands as attributes.

## Stores

| Store | Remember |
|---|---|
| Managed KB | No store to run. Current default for new work |
| S3 Vectors | Vector bucket, low ops |
| OpenSearch Serverless | VECTORSEARCH collection. Create collection and index **before** the KB. Field names in the index must match the KB config. Data access policy must include the KB role |
| OpenSearch managed | Existing cluster, k-NN / neural plugin |
| Aurora or RDS pgvector | Extension on, dimensions match, credentials in Secrets Manager, SQL filters |
| Pinecone, Redis, MongoDB Atlas | You already pay for them. Secrets in Secrets Manager. Dimensions match |
| Neptune Analytics | Graph RAG |
| DynamoDB | Metadata and session. Not the ANN index |
| Kendra | Different product: connectors and ACLs, less of this pipeline |

Hybrid (`overrideSearchType=HYBRID`) is for OpenSearch Serverless, RDS/Aurora, and MongoDB Atlas with a filterable text field. Everyone else: semantic.

IAM for a customer-managed KB role: trust `bedrock.amazonaws.com` with confused-deputy conditions, read the corpus bucket, write the vector store, `bedrock:InvokeModel` on the embedding model, KMS, and `secretsmanager:GetSecretValue` if the store has a password. New roles are eventually consistent. A create that fails once may succeed on retry.

## Query techniques

| Technique | Helps |
|---|---|
| Metadata filter | Precision and isolation |
| Hybrid | Exact tokens plus meaning |
| Query rewrite | Vague or slang questions |
| Decomposition | Multi-hop questions |
| HyDE | Query looks nothing like the docs. Embed a hypothetical answer |
| Rerank | Top-k order is wrong, recall is acceptable |
| Smaller k after rerank | Context window and faithfulness |
| MMR / dedupe | Five chunks of the same paragraph |
| Citations | Trust and debugging. RetrieveAndGenerate returns them |
| `sessionId` | Multi-turn RAG. Omit it and turn 2 has no memory |

Rerank is a second hop on a short list (for example 50 → 5). It is not a reason to skip chunking.

## Freshness

| Corpus | Pattern |
|---|---|
| Static policy, monthly | Scheduled ingestion |
| S3 drops through the day | Event → `StartIngestionJob` or incremental sync |
| OLTP rows | Streams → embed → upsert |
| Deletes | Deletion policy that removes vectors. `RETAIN` keeps citing deleted files |

Monitor lag: newest source timestamp vs newest indexed timestamp.

## Failure decoder

| Observation | Fix the |
|---|---|
| Right document, wrong slice | Chunking or hierarchical parent |
| Never retrieves the document | Ingestion, filter, embedding model mismatch, wrong index |
| Retrieves it as rank 30 | Hybrid, rerank, or the embedding is a bad fit |
| Cites last year’s PDF | Freshness or metadata date filter |
| Other tenant’s text | Filter, index-per-tenant, or IAM on the source |
| Numbers from a table are scrambled | Parser |
| Answer ignores a good chunk | Prompt, lost-in-the-middle, k too high, grounding |
| Ingestion fails on dimension | Index mapping ≠ embedding model |

## Security reminders

- Encrypt the bucket and the index with a CMK when required.
- Scope the KB role to the prefix, not the account.
- `Retrieve` is a CloudTrail data event.
- Retrieved text is untrusted input (indirect prompt injection). Run the prompt-attack filter on it.
- Per-user ACLs: filter metadata at query time. A shared KB role that reads everything will over-retrieve if the app forgets the filter.
