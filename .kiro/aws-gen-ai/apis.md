---
inclusion: auto
name: aws-gen-ai-apis
description: "Use when calling Amazon Bedrock APIs or writing IAM for them: Converse, ConverseStream, InvokeModel, ApplyGuardrail, Retrieve, RetrieveAndGenerate, InvokeAgent, endpoints, and inference-profile permissions."
---

# Bedrock APIs

Call the plane that owns the operation. The wrong endpoint returns `UnknownOperationException` or does nothing useful.

| Endpoint | Plane | Calls |
|---|---|---|
| `bedrock` | Control | `ListFoundationModels`, `GetFoundationModel`, inference profiles, guardrail CRUD, provisioned throughput, model invocation logging |
| `bedrock-runtime` | Data | `Converse`, `ConverseStream`, `InvokeModel`, `InvokeModelWithResponseStream`, `ApplyGuardrail` |
| `bedrock-mantle` | Data | OpenAI-compatible Chat Completions, Responses, and Messages for clients that already speak that shape |
| `bedrock-agent` | Agent and Knowledge Base control | Agents, action groups, knowledge bases, data sources, `PrepareAgent`, aliases |
| `bedrock-agent-runtime` | Agent and Knowledge Base data | `InvokeAgent`, `Retrieve`, `RetrieveAndGenerate` |
| `bedrock-agentcore-control` | AgentCore control | Runtimes, gateways, harnesses |
| `bedrock-agentcore` | AgentCore data | Invoke |

Model calls go to `bedrock-runtime`. Retrieval and classic agent calls go to `bedrock-agent-runtime`. Preparing an agent goes to `bedrock-agent`.

## Converse

Use Converse unless you need a provider-only feature. One schema covers every model. InvokeModel is provider JSON (Anthropic, Titan, Llama, and Nova differ). A mismatched InvokeModel body is the usual “malformed input request.”

```json
{
  "modelId": "us.anthropic.claude-sonnet-... or an inference-profile id",
  "system": [{ "text": "You are ..." }],
  "messages": [
    { "role": "user", "content": [{ "text": "..." }] },
    { "role": "assistant", "content": [{ "text": "..." }] }
  ],
  "inferenceConfig": {
    "maxTokens": 1024,
    "temperature": 0.2,
    "topP": 0.9,
    "stopSequences": ["END"]
  },
  "toolConfig": {
    "tools": [{ "toolSpec": { "name": "getOrder", "inputSchema": { "json": {} } } }]
  },
  "guardrailConfig": {
    "guardrailIdentifier": "id",
    "guardrailVersion": "1",
    "trace": "disabled"
  }
}
```

Roles in `messages` are `user` and `assistant` only. Keep the system prompt in the top-level `system` field so selective guardrail evaluation can tell instructions from user text.

Set `maxTokens` every time. Unset, the call reserves the model maximum and can throw `ThrottlingException` at a low request rate.

The CLI can call `converse`. Streaming is SDK-only (`ConverseStream`).

Stop reasons you will handle: `end_turn`, `max_tokens`, `stop_sequence`, `tool_use`, `guardrail_intervened`.

Usage: `inputTokens`, `outputTokens`, `cacheReadInputTokens`, `cacheWriteInputTokens`.

### Prompt cache marker

After the stable prefix:

```json
{ "cachePoint": { "type": "default" } }
```

Where the model allows a one-hour TTL: `"ttl": "1h"`. When both a longer and a shorter TTL are used, the longer one is placed first. Five minutes is the broad default.

### InvokeModel

Only when Converse cannot express the feature. Match the provider.

- Anthropic Messages: `anthropic_version`, `messages`, `max_tokens`.
- Titan text: `inputText`, `textGenerationConfig`.
- Embeddings: `inputText` for Titan, `texts` / `input_type` for Cohere. Mixing those is the malformed-body bug.

Newer Claude versions removed assistant-message prefill. Ending the request on an assistant turn fails validation on those models.

## Guardrail calls

| Mode | Where |
|---|---|
| The whole turn, input and output | `guardrailConfig` on Converse |
| Selected blocks | `guardContent` inside content. Word filters still scan the whole request |
| No model call | `ApplyGuardrail` on `bedrock-runtime`, `source` of `INPUT` or `OUTPUT` |

Create and version on the control plane (`CreateGuardrail`, `CreateGuardrailVersion`). Invoke with a number, not `DRAFT`.

Streaming accepts `streamProcessingMode`. `sync` checks, then emits. `async` emits first, can deliver text the policy would have blocked, and does not anonymize PII. Compliance workloads use `sync`.

## IAM

| Action | Allows |
|---|---|
| `bedrock:InvokeModel` | Synchronous inference |
| `bedrock:InvokeModelWithResponseStream` | Streaming. Forgetting it looks like “the console works and the app is denied” |
| `bedrock:ApplyGuardrail` | Standalone guardrail |
| `bedrock:Retrieve` | Knowledge Base retrieve |
| `bedrock:RetrieveAndGenerate` | Knowledge Base generate |
| `bedrock:InvokeAgent` | Classic agent |
| `bedrock:CreateGuardrail` and the other control actions | Admin, not the application role |

Condition key `bedrock:GuardrailIdentifier` denies `InvokeModel` that omits that guardrail.

Resource ARNs:

- Foundation model: `arn:aws:bedrock:region::foundation-model/model-id` (the account segment is empty).
- Inference profile: `arn:aws:bedrock:region:account:inference-profile/id`.
- Cross-Region also needs `arn:aws:bedrock:*::foundation-model/...`.

Application roles are scoped to the model ARNs and knowledge base ARNs they use. `bedrock:*` does not belong on an application role.

## Retry

```text
retries = { max_attempts: 5, mode: "adaptive" }
```

Retry throttling, model timeout, service unavailable, and internal errors. Do not retry validation, access denied, or not found.

## Logging

Model invocation logging is configured on the control plane (`PutModelInvocationLoggingConfiguration`) and writes prompts and completions to CloudWatch Logs, S3, or both. That is the forensic record. Encrypt it, restrict who can read it, and set a retention period. It can contain PII even when a guardrail anonymized the API response.
