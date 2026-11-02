# Evaluation metrics

Open-ended generation does not have a single accuracy. Name the layer you measured.

## Generation

| Metric | Meaning | Good for | Weak for |
|---|---|---|---|
| Relevance | Answer addresses the question | Q&A | Does not check truth |
| Faithfulness / groundedness | Claims are supported by the supplied context | RAG | Useless with no context |
| Factuality | Claims match the real world | Closed book | Expensive to judge |
| Consistency | Same meaning across runs or across a story | Agents, long answers | Too strict if you require identical strings |
| Fluency | Grammar and readability | User-facing text | A fluent hallucination scores high |
| Completeness | Required points present | Rubrics, extractions | — |
| Conciseness | No padded junk | Cost and UX | — |
| Toxicity / harm | Unsafe content rate | Safety gate | Not a quality score |
| JSON validity / schema pass | Parseable contract | Pipelines | Says nothing about truth |
| Exact match / F1 | Token overlap with a gold string | Extraction, classification, short spans | Chat |
| BLEU / ROUGE | N-gram overlap with a reference | Translation, some summaries | Chat, RAG. High ROUGE can still be wrong |
| Semantic similarity / BERTScore | Embedding distance to a reference | Paraphrase is allowed | Can score a fluent wrong answer as close |
| LLM-as-judge | Another model applies a rubric | Scale | Judge bias, position bias, self-preference. Sample with humans |
| Human rubric | People score a scale | High stakes, tone, policy | Slow, expensive |
| Thumbs / acceptance | Production feedback | What users do | Selection bias, sparse |
| Hallucination rate | Fraction of answers with an unsupported claim | Executive metric | Define the rubric or the number is fake |

Temperature does not appear in the table. It is a generation parameter, not a metric.

## Retrieval

Computed on a labeled set: for each query, which documents or chunks are relevant.

| Metric | Meaning |
|---|---|
| Hit rate / recall@k | Was at least one relevant chunk in the top k? Recall@k is the fraction of all relevant chunks that appear in the top k |
| Precision@k | Fraction of the top k that are relevant |
| MRR | Mean of `1 / rank` of the first relevant hit. Rewards “correct at position 1” |
| nDCG | Graded relevance and position. Use when some docs are more relevant than others |
| Context precision | Retrieved set is mostly useful (RAG name for precision of the context you actually stuffed in the prompt) |
| Context recall | The facts needed for the gold answer were in the retrieved set |
| Empty-result rate | Filter or threshold returned nothing |
| Latency | p50/p95 of the search, separate from model latency |

k is a product choice. Larger k raises recall and hurts precision, cost, and the context window.

## RAG, four numbers

| | Context relevant | Context not relevant |
|---|---|---|
| Answer follows context | Good RAG, or faithfully wrong if the doc is wrong | Faithful to junk. Fix retrieval |
| Answer ignores context | Hallucination despite evidence. Fix prompt or grounding | Worst case |

Practical quartet used in most write-ups (RAGAS-style names are fine if you define them):

1. **Context recall** — retrieval found the needed facts.
2. **Context precision** — retrieval did not bury them in noise.
3. **Faithfulness** — the answer sticks to that context.
4. **Answer relevance** — the answer addresses the user, not a nearby topic.

Cite them separately. “RAG accuracy 80%” hides which stage broke.

## Agents

| Metric | Meaning |
|---|---|
| Task completion / success rate | End state matches the goal under the rubric |
| Tool selection accuracy | The right tool was chosen |
| Argument accuracy | Parameters valid and correct |
| Steps to completion | Efficiency. Loops show up here |
| Tool error rate | Lambda or API failures |
| Policy adherence | Confirmations happened, forbidden tools did not run |
| Trace quality | A human can follow the trace. Debugging signal |

Bedrock agent evaluation jobs automate task success against conversations you supply. A completion that called the wrong tool and got lucky is not a pass if tool choice is in the rubric.

## Safety and responsibility

| Metric | How |
|---|---|
| Jailbreak success rate | Adversarial set. Must be near zero for the gate |
| PII leakage rate | Detector on outputs (and on logs if that is the scope) |
| Refusal accuracy | Refuses harmful, does not refuse benign (false-refusal rate) |
| Grounding block rate | Guardrail contextual grounding interventions. Spike = threshold too high or retrieval collapse |
| Bias / slice gap | Metric computed per group, locale, or product line. Clarify for traditional models. Sliced human or judge scores for generative |
| Uncertainty | Self-reported confidence is weak. Prefer retrieval score, grounding score, or agreement across samples |

## Cost and performance (put them next to quality)

| Metric | Why it is on the scorecard |
|---|---|
| Input / output tokens per task | The bill |
| Cache hit rate | `cacheRead / (cacheRead + cacheWrite)` plus skipped |
| Latency to first token | Perceived speed |
| End-to-end latency | Includes retrieval and tools |
| Price per successful task | Stops a “cheaper model” that fails twice as often |
| Throughput | Tokens per minute vs quota |

A model that is 10% more faithful and 4× the tokens needs a business call. The exam expects that comparison in the eval, not a quality-only table.

## Experiment hygiene

- Freeze the dataset. Do not tune on the only set you report.
- Record model id, prompt version, temperature, k, chunk config, and date.
- Canary before full shift. Roll back the alias, not the dashboard.
- String equality is valid for classification labels and invalid for paragraphs.
- Calibrate LLM-as-judge on a human sample before it becomes a release gate.
- Separate safety regressions from quality regressions so a style change does not hide a jailbreak regression.
