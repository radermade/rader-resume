# Bedrock API cheat sheet

Five planes. The wrong endpoint is `UnknownOperationException` or a silent no-op.

| Endpoint | Plane | Calls you should know |
|---|---|---|
| `bedrock` | Control | `ListFoundationModels`, `GetFoundationModel`, inference profiles, guardrail CRUD, provisioned throughput, model invocation logging config |
| `bedrock-runtime` | Data | `Converse`, `ConverseStream`, `InvokeModel`, `InvokeModelWithResponseStream`, `ApplyGuardrail` |
| `bedrock-mantle` | Data | OpenAI-compatible Chat Completions, Responses, Messages. Newer clients. Exam scenarios still center on Converse |
| `bedrock-agent` | Agent control | Create/update agent, action group, KB, data source, `PrepareAgent`, aliases |
| `bedrock-agent-runtime` | Agent data | `InvokeAgent`, `Retrieve`, `RetrieveAndGenerate` |

AgentCore is separate: `bedrock-agentcore-control` (create runtimes, gateways, harnesses) and `bedrock-agentcore` (invoke). A gateway has its own host `{gatewayId}.gateway.bedrock-agentcore`.

## Converse vs InvokeModel

Use **Converse** unless you need a provider-only feature.

| | Converse | InvokeModel |
|---|---|---|
| Body | One schema for every model | Provider JSON (Anthropic ≠ Titan ≠ Llama ≠ Nova) |
| Messages | `role` + `content[]` blocks | Whatever the provider invented |
| System | Top-level `system` | Often a special message or field |
| Tools | `toolConfig`, `toolUse`, `toolResult` | Provider tool format |
| Guardrail | `guardrailConfig` | Also supported, different shape |
| Streaming | `ConverseStream` | `InvokeModelWithResponseStream` |
| Wrong body | Rare | “Malformed input request” |

CLI can call `converse`. CLI cannot stream. Streaming is SDK-only (`converse_stream` / `ConverseStreamCommand`).

### Converse shape (logical)

```json
{
  "modelId": "us.anthropic.claude-sonnet-... or inference-profile id",
  "system": [{ "text": "You are ..." }],
  "messages": [
    { "role": "user", "content": [{ "text": "..." }] },
    { "role": "assistant", "content": [{ "text": "..." }] }
  ],
  "inferenceConfig": { "maxTokens": 1024, "temperature": 0.2, "topP": 0.9, "stopSequences": ["END"] },
  "toolConfig": { "tools": [{ "toolSpec": { "name": "getOrder", "inputSchema": { "json": {} } } }] },
  "guardrailConfig": { "guardrailIdentifier": "id", "guardrailVersion": "1", "trace": "disabled" }
}
```

Roles in `messages` are `user` and `assistant` only. Do not put the system prompt as a user message if you also need guardrail behavior that skips trusted instructions. See guardrails doc for `guardContent`.

`maxTokens` is required in practice. Unset → model maximum reserved against quota → `ThrottlingException`.

Usage block: `inputTokens`, `outputTokens`, `cacheReadInputTokens`, `cacheWriteInputTokens`.

Stop reasons you will see: `end_turn`, `max_tokens`, `stop_sequence`, `tool_use`, `guardrail_intervened`.

### Tool turn

1. You send `toolConfig` and the user message.
2. Response content has `toolUse` (`toolUseId`, `name`, `input`).
3. You run the tool **yourself**.
4. Next message is `user` with `toolResult` (`toolUseId`, output).
5. Model answers.

The model does not execute Lambda by itself on Converse. Classic **Agents** execute action-group Lambdas for you. Return-of-control agents behave like step 3.

### Prompt cache marker

A content block, after the stable prefix:

```json
{ "cachePoint": { "type": "default" } }
```

One-hour TTL where the model allows it: `"ttl": "1h"`. Metrics as above. Zero on both means unsupported model, under the token minimum, or the prefix changed.

## InvokeModel

Only when Converse cannot express the feature. Match the provider:

- Anthropic Messages: `anthropic_version`, `messages`, `max_tokens`.
- Titan text: `inputText`, `textGenerationConfig`.
- Embeddings: `inputText` (Titan) vs `texts` / `input_type` (Cohere). Mixing these is the malformed-body bug.

## Guardrail APIs

| Mode | Where |
|---|---|
| All messages on this call | `guardrailConfig` on Converse |
| Only some blocks | `guardContent` blocks inside content |
| No model call | `ApplyGuardrail` on `bedrock-runtime` (`source` = `INPUT` or `OUTPUT`) |

Create and version on the control plane: `CreateGuardrail`, `CreateGuardrailVersion`. Invoke with a number, not `DRAFT`.

## Knowledge base APIs

| API | Client | Returns | Notes |
|---|---|---|---|
| `CreateKnowledgeBase`, `CreateDataSource`, `StartIngestionJob` | `bedrock-agent` | Control | Ingestion is async. Poll the job |
| `Retrieve` | `bedrock-agent-runtime` | Chunks + scores + metadata | You write the prompt |
| `RetrieveAndGenerate` | `bedrock-agent-runtime` | Answer + citations + `sessionId` | Managed RAG. Customer-managed KB path. Pass `sessionId` on the next turn |
| `AgenticRetrieveStream` | SDK, managed KB | Multi-step retrieve | Streaming, not the CLI |

`Retrieve` filters use the KB filter language (`equals`, `in`, `andAll`, `orAll`, …), not OpenSearch JSON, even if OpenSearch is the store.

`overrideSearchType`: `HYBRID` or `SEMANTIC` only on stores that support it.

Guardrail on `RetrieveAndGenerate` is opt-in: `guardrailConfiguration` with id and version.

CloudTrail: `Retrieve` and `RetrieveAndGenerate` are **data events**. Default trails miss them.

## Agent APIs

| API | Client | When |
|---|---|---|
| `CreateAgent`, `CreateAgentActionGroup`, `AssociateAgentKnowledgeBase`, `PrepareAgent` | `bedrock-agent` | Build |
| `CreateAgentAlias` | `bedrock-agent` | Production pointer to a prepared version |
| `InvokeAgent` | `bedrock-agent-runtime` | Run. `sessionId`, `inputText`, optional `enableTrace` |
| Stream events | same | Chunks, trace, return-of-control payload |

Return of control: the stream emits the API name and parameters and pauses. Your process returns the tool result on the next `InvokeAgent`. If nobody does, the agent never finishes.

`enableTrace` is for debugging and for “show your steps” UIs. It is verbose and may contain sensitive tool payloads.

## IAM actions to recognize

| Action | Allows |
|---|---|
| `bedrock:InvokeModel` | Sync inference |
| `bedrock:InvokeModelWithResponseStream` | Streaming. Easy to forget. Console works, app gets denied |
| `bedrock:ApplyGuardrail` | Standalone guardrail |
| `bedrock:Retrieve` | KB retrieve |
| `bedrock:RetrieveAndGenerate` | KB RAG |
| `bedrock:InvokeAgent` | Agent |
| `bedrock:CreateGuardrail` and friends | Admin, not the app role |

Condition key `bedrock:GuardrailIdentifier` to require a guardrail. Resource ARNs:

- Foundation model: `arn:aws:bedrock:region::foundation-model/model-id` (account empty)
- Inference profile: `arn:aws:bedrock:region:account:inference-profile/id`
- Cross-Region also needs `arn:aws:bedrock:*::foundation-model/...`

## SDK retry

```text
retries = { max_attempts: 5, mode: "adaptive" }
```

Retry throttling, model timeout, service unavailable, internal. Do not retry validation or access denied.

## Logging APIs

Model invocation logging is configured on the **control** plane (`PutModelInvocationLoggingConfiguration`) and writes prompts and completions to CloudWatch Logs and/or S3. That is the forensic log. It is also the PII leak if you do not encrypt and retain it properly.
