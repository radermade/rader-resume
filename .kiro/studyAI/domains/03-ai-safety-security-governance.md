# Domain 3 — AI safety, security, and governance

**20% of scored content.** Tasks 3.1–3.4. Second-highest leverage after Domain 1. Questions stack controls: filter, ground, authorize, log, explain. One control is rarely the whole answer.

Full guardrail mechanics: [guardrails.md](../reference/guardrails.md).

## 3.1 Input and output safety

Defense in depth. Do not stop at one layer.

| Layer | What it stops | Tool |
|---|---|---|
| Edge | Volumetric abuse, known bad payloads | WAF on API Gateway, throttling |
| Pre-process | Obvious PII, toxicity, injection before the model spends tokens | Comprehend, Lambda rules, `ApplyGuardrail` |
| Model call | Harmful input and output together | Guardrail on Converse (`guardrailConfig`) |
| Grounding | Answers that are not in the sources | Knowledge Base citations, contextual grounding check |
| Post-process | Schema, leaked secrets, policy phrases | Lambda, JSON Schema, a second classifier |
| Deterministic path | Math, billing, eligibility | Do **not** let the model invent the query. Text-to-SQL only with a constrained schema, or skip the model and use SQL |

### Guardrail capabilities (short)

- Content filters: hate, insults, sexual, violence, misconduct, **prompt attack**.
- Denied topics: your definitions (“do not discuss pricing exceptions”).
- Word filters and managed profanity lists.
- PII: `BLOCK` (fail the call) or `ANONYMIZE` (replace with a placeholder). Per entity type.
- Regex for custom patterns (account numbers).
- Contextual grounding: grounding score vs sources, relevance score vs the question.
- Automated reasoning checks: logical rules over the answer.

`BLOCK` when leakage is unacceptable. `ANONYMIZE` when the user still needs the rest of the answer.

### Hallucinations

| Technique | Role |
|---|---|
| RAG with citations | Model must stick to retrieved text |
| Contextual grounding thresholds | Block if the answer is not supported or not relevant. Start near 0.7 and tune |
| Confidence / similarity | Low score → refuse or escalate to a human |
| JSON Schema | Stops malformed output, **not** false facts |
| Lower temperature | More stable wording. Does not add knowledge |
| “Answer only from context. If missing, say you do not know.” | Necessary prompt. Not sufficient alone |

### Prompt injection and jailbreaks

- Guardrail **prompt attack** filter on user text and on retrieved text.
- Retrieved documents and tool results are untrusted. Indirect injection hides in a PDF.
- If you use selective `guardContent` blocks, wrap **all** untrusted blocks (user, tools, chunks). Word filters still scan everything; content, topic, PII, and grounding filters honor the blocks.
- Separate system instructions from user content. Do not concatenate them into one string with no boundary.
- Least privilege on tools so a successful injection cannot `iam:*` or delete a bucket.
- Adversarial tests in the pipeline (Domain 5), not a one-time demo.

Streaming: `sync` holds chunks until the guardrail passes. `async` streams first and can leak PII or harmful text, and it does **not** anonymize PII. Compliance workloads use `sync`.

**Pick this when:** “block competitor talk and mask emails” → one guardrail with a denied topic and PII anonymize. “check a document before any model call” → `ApplyGuardrail`. “answers invent policies” → RAG + grounding, not a content filter (content filters do not know your policy).

## 3.2 Data security and privacy

### Network and access

- Interface VPC endpoints for Bedrock. Traffic stays on PrivateLink.
- IAM roles, scoped to model ARNs and KB ARNs. No `bedrock:*` on an app role.
- Lake Formation when the source is a governed data lake and row/column permissions matter.
- CloudWatch / CloudTrail on who read what. See logging gap below.
- KMS customer-managed keys for buckets, logs, guardrail config, and KB storage when the stem says compliance or “customer-managed key.”

### Bedrock data handling (exam facts)

- AWS does not use your prompts and completions to train base models.
- On-demand inference stays in the Region, except cross-Region profiles. Geo profiles (`eu.`) stay in-geography. `global.` does not promise that.
- Model vendors do not get your data to improve their models through Bedrock’s standard agreement. Do not “solve privacy” by self-hosting unless the stem says the data cannot go to Bedrock at all.

### PII

| Tool | Use |
|---|---|
| Comprehend PII | Detect and locate entity types in text you process yourself |
| Macie | Discover PII in **S3** (logs, data lakes, exports) |
| Guardrail sensitive information policy | Block or anonymize during inference |
| S3 Lifecycle | Expire prompts, logs, and training extracts. Privacy means not keeping data forever |
| Masking / tokenization before the prompt | The model never sees the raw value. Stronger than anonymize-after |

**Logging gap:** guardrail anonymization changes the **API response**. Model invocation logs can still store the raw prompt and completion, including PII, in CloudWatch or S3. For HIPAA/GDPR: encrypt the log group with KMS, least-privilege IAM, retention limits, Macie on exported logs, or turn invocation logging off for that workload. `trace: enabled` on a guardrail returns the matched sensitive text in the response. Production uses `trace: disabled`.

### Privacy patterns that keep utility

- Anonymize identifiers, leave the medical or financial **situation** if the task needs it.
- Retrieve with a tenant filter so the model never sees other users’ chunks.
- Do not put secrets in the system prompt. Secrets Manager, and the tool fetches at runtime.

**Pick this when:** “find PII already sitting in the data lake” → Macie, not a guardrail (the guardrail only sees inference traffic). “stop the model repeating a social security number” → guardrail PII `BLOCK` or `ANONYMIZE`, plus log encryption. “column-level access on the lake that feeds RAG” → Lake Formation.

## 3.3 Governance and compliance

| Obligation | Artifact |
|---|---|
| What the model is and is not | Model card (SageMaker or a document you generate). Limits, intended use, data |
| Where a fact came from | Glue Data Catalog + metadata tags on chunks. Citations in the answer |
| Who changed a prompt or guardrail | CloudTrail |
| Why this answer | Invocation logs, agent trace, prompt version id |
| Lineage of training or index data | Glue, S3 tags, catalog |
| Ongoing misuse, drift, bias | Scheduled evals, CloudWatch alarms, a remediation workflow (ticket, rollback, disable alias) |

Organizational governance the exam wants in words: policy (what is allowed), ownership (who approves prompts and models), inventory (which apps call which models), and evidence (logs and eval reports). The Well-Architected Generative AI Lens is the checklist they name.

Continuous controls: token redaction in logs, automated alerts on toxicity or grounding failures, bias drift checks, quarantine a model version that fails a quality gate.

**Pick this when:** “auditor asks which PDF a sentence came from” → citations + source metadata + catalog, not CloudWatch CPU. “who invoked the model” → CloudTrail **data events** for `InvokeModel` (management events do not record every invocation) and invocation logs for the payload.

## 3.4 Responsible AI

AWS framing you should be able to map to a service:

| Principle | In a design answer |
|---|---|
| Fairness | Eval slices by demographic or locale. SageMaker Clarify for classic models. LLM-as-judge or human review for generative output. A/B through Prompt Management or Prompt Flows |
| Explainability | Show citations, reasoning traces (agent trace), confidence. Do not claim chain-of-thought is a faithful explanation of weights |
| Privacy | Section 3.2 |
| Robustness | Adversarial tests, injection filters, eval regression |
| Governance | Section 3.3 |
| Transparency | User-visible “AI generated,” source list, limits on the model card |
| Controllability | Human approval for high-impact actions (Augmented AI, Step Functions callback) |

**Amazon Augmented AI (A2I)** is the human-review loop for predictions. Use it when a person must accept or edit the output before it hits a customer or a system of record. Ground Truth is for building labeled eval or training sets, not for runtime approval.

Fairness on this exam is rarely “retrain with a new loss.” It is: define the slice, measure, compare prompts or models, block a deploy that regresses, document the limit.

**Pick this when:** “user must see why” → citations and/or agent trace. “loan denial must be reviewed by a person” → A2I or an approval state, not a lower temperature. “did quality change for a region or group” → sliced eval, CloudWatch metric per slice.

## Domain 3 traps

- A system prompt that says “be safe” as the only control.
- Content filter chosen to fix hallucinations.
- Guardrail `DRAFT` in production, or id without version.
- Assuming anonymize cleans the logs.
- Async streaming guardrail on a PCI or HIPAA bot.
- Macie and Comprehend swapped. Macie = S3. Comprehend = text you send it.
- CloudTrail management events alone as proof of every prompt.
- Fine-tuning to “remove bias” when the stem asks for measurement and a review step.
- `global.` cross-Region profile for a residency requirement.
