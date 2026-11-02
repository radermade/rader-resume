---
inclusion: auto
name: aws-gen-ai-models
description: "Use when selecting, configuring, routing, customizing, or swapping Amazon Bedrock or SageMaker foundation models: model size, modality, inference profiles, provisioned throughput, fine-tuning, LoRA, and Model Registry."
---

# Models

Pick the smallest model that meets the quality bar on the eval set. Record model id, prompt version, latency, and tokens with every result.

## Selection

| Need | Use |
|---|---|
| Hard reasoning, long instructions | A larger Claude (Opus or Sonnet class) |
| Cheap, fast, high volume | Haiku, Nova Micro, or Nova Lite |
| Text and image | Claude or Nova |
| Video | A Nova model that lists video |
| Open weights that Bedrock can customize | Llama, or whichever model Bedrock currently allows you to customize |
| Embeddings | Titan Text Embeddings V2 (dimensions are configurable). Cohere Embed when multilingual quality is the requirement |
| Reorder a retrieved list | A Bedrock reranker (Cohere Rerank or Amazon Rerank). Match the one you standardize on |

Check live capabilities with `ListFoundationModels`. Trade off quality, cost, latency, modality, customization, and context window. Verify model ids in the account before baking them into infrastructure.

Some models reject a bare on-demand id. `ValidationException: on-demand throughput isn't supported` means call an inference profile id (`us.`, `eu.`, `apac.`, `global.`) or a provisioned-throughput ARN.

## Model id shapes

| Access | Shape |
|---|---|
| On-demand, one Region | `provider.model-name` |
| Cross-Region inference | `geo.provider.model-name` |
| Application inference profile | `arn:...:inference-profile/id` |
| Provisioned throughput | `arn:...:provisioned-model/id` |

`us.`, `eu.`, and `apac.` stay inside that geography. `global.` may leave it. Residency requirements use the geo prefix.

Cross-Region IAM allows both the inference-profile ARN and the foundation-model ARN with a wildcard Region (`arn:aws:bedrock:*::foundation-model/...`). A single-Region model ARN denies the call when Bedrock routes it.

## Routing

The application calls one interface. The model id lives in AppConfig, Parameter Store, or a config table, resolved in the gateway. That is how providers change without a deploy of business code.

- **Static.** A route maps to a model (`/summarize` → a small model).
- **Dynamic.** The gateway or Step Functions picks from content, complexity, or live error rate.
- **Cascade.** A small model answers first. Escalate when confidence is low or the task is hard.

Most questions in a mixed workload are easy. Pay for the flagship model on the hard slice.

## Resilience

- Cross-Region inference when a model is thin in one Region or the workload needs more burst, inside the allowed geography.
- A circuit breaker (Step Functions, counting failures) stops calling and fails over or degrades.
- Degradation paths: a smaller model, a cached answer, a clear retry message, or a non-model path.
- Retry throttling, timeouts, and 5xx with the SDK adaptive mode, jitter, and a cap. Validation and access denied are bugs, not transient errors.
- Roll back with a Model Registry version and an alias. Automate promote and revert. Keep the last good endpoint.

## Customization

Prompting is the first attempt. Retrieval is how facts stay current. Customization changes weights.

| Technique | What changes | Use when |
|---|---|---|
| Prompting | Nothing in the weights | First choice |
| Retrieval | The context window | Facts must be current and cited |
| Fine-tuning | Behavior, format, style | Labeled input/output pairs, and the task is stable |
| Continued pre-training | Domain language in the weights | Unlabeled domain corpus, new vocabulary |
| LoRA / adapters | Small trainable matrices on a frozen model | Domain adaptation with less compute. Often SageMaker |
| Full fine-tune | More of the weights | Last resort. Cost and catastrophic forgetting |

Training a model from scratch is a different job. This stack selects, adapts, and ships.

Lifecycle: adapt → register in SageMaker Model Registry → approval → deploy to a SageMaker endpoint or Bedrock custom-model throughput → canary → rollback → retire old versions. Bedrock custom models usually require provisioned throughput to invoke. On-demand is for serverless base models.

Ship a model Bedrock does not host on SageMaker. Keep the general model on Bedrock when it already meets the task.

## Parameters

- Set `maxTokens` on every call. An unset cap reserves the model maximum and throttles at a low request rate.
- Low temperature for extraction, classification, and anything that feeds a schema.
- top-p and top-k change diversity. They are not a latency or truth control.
- Temperature does not add knowledge. A lower temperature makes wording more stable.

## Input shape

The model is only as good as the input.

| Job before the model | Service |
|---|---|
| Quality rules on tables (completeness, range, freshness) | Glue Data Quality |
| Visual prep of tabular data | SageMaker Data Wrangler |
| Row-level checks | Lambda |
| Alert on a bad batch | CloudWatch |
| OCR, forms, tables | Textract |
| Speech to text | Transcribe |
| Entities, sentiment, PII on a string you submit | Comprehend |
| Image labels and moderation | Rekognition |
| Heavy batch transform | SageMaker Processing |
| The requirement is one multimodal understanding call | A Bedrock multimodal model |

Converse takes `messages` of `user` and `assistant`, an optional top-level `system`, and `inferenceConfig`. InvokeModel takes provider-specific JSON. A Titan body sent to a Claude model fails validation. Dialog roles alternate. A SageMaker endpoint consumes that container’s contract, which is a different body from Converse.

Normalize entities, units, and dates before the main prompt. A short rewrite into the expected schema is cheaper than a large model guessing at messy text.
