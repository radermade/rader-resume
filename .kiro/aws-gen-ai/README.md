---
inclusion: manual
name: aws-gen-ai
description: "Map of the AWS generative AI steering files. Include when you need the index, not the rules themselves."
---

# AWS generative AI steering

Build rules for Kiro. Each file is one decision area. They load on their own when the task matches (`inclusion: auto`). This index is manual.

The rules are product choices: which pattern, which service, and why. Model ids, quotas, and default chunk sizes change. Verify those in the account. The choice is the stable part.

Facts were distilled from `.kiro/studyAI` and rewritten as steering. That folder stays the study base.

## Files

| File | Follow it when you are |
|---|---|
| [decisions.md](decisions.md) | Choosing the pattern before naming a product |
| [models.md](models.md) | Selecting, routing, or customizing a model |
| [prompts.md](prompts.md) | Writing, versioning, or chaining prompts |
| [retrieval.md](retrieval.md) | Designing ingestion, search, or a knowledge base |
| [agents.md](agents.md) | Adding tools, a loop, or a human approval |
| [apis.md](apis.md) | Calling Bedrock or scoping IAM for it |
| [safety.md](safety.md) | Guardrails, privacy, audit, responsible AI |
| [runtime.md](runtime.md) | Hosting, streaming, queues, residency, CI |
| [operations.md](operations.md) | Cost, latency, evals, and failures |
| [services.md](services.md) | Assigning a job to one AWS service |

## How Kiro loads them

Workspace steering is `.kiro/steering/`. Each file here is linked from there as `aws-gen-ai-<name>.md`, so auto inclusion works in this repo. Editing a file here edits the steering Kiro sees.

Reference one in chat with `#aws-gen-ai-decisions` (the link name).
