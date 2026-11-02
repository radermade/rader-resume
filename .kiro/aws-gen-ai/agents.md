---
inclusion: auto
name: aws-gen-ai-agents
description: "Use when building agents or tool-using systems on AWS: AgentCore Harness, Runtime, Gateway, Memory, classic Bedrock Agents, Strands, Agent Squad, MCP, Step Functions orchestration, return of control, and human approval."
---

# Agents and tools

An agent is a loop: the model plans, calls a tool, reads the observation, and repeats until it stops. A completion with no tool and no branch is a prompt. Steps that are known before the request arrives are a workflow, which is easier to audit.

## Where a new agent runs

| Situation | Build it on |
|---|---|
| Managed loop. Model, tools, memory, and instructions are config. No container | AgentCore Harness |
| You already wrote the loop and need a host | AgentCore Runtime (HTTP or MCP). The platform requires a real session id; short ids are rejected |
| Existing REST APIs should show up as tools | AgentCore Gateway. It exposes them as MCP tools, so you do not write a tool server. A gateway host looks like `{gatewayId}.gateway.bedrock-agentcore` |
| Several specialists, and a classifier picks one | Agent Squad, or a supervisor whose specialists have narrow tools |
| The path must be a diagram: retries, fan-out, a human, a circuit breaker | Step Functions |
| The running system already has action groups, aliases, and `PrepareAgent` | Classic Bedrock Agents. Keep it there. New systems do not start there |
| Agent and tool behavior written as code | Strands Agents, hosted on AgentCore Runtime when you need a managed host |

Control-plane calls for AgentCore go to `bedrock-agentcore-control`. Invoke goes to `bedrock-agentcore`.

A Harness migration from a classic agent is not behavior-identical. Return of control and how the guardrail is attached are the pieces to re-test.

## Classic Bedrock Agents

Maintain these rules on any system that still uses them.

| Piece | Rule |
|---|---|
| Action group | OpenAPI 3 schema, plus Lambda fulfillment or return of control |
| Knowledge base | A built-in retrieval tool, with its own instructions |
| Guardrail | Identifier and a numbered version. Missing either one means the guardrail is off. `DRAFT` is not a production version |
| Memory | A session summary, so the next turn is cheaper than replaying the transcript |
| Code interpreter | Sandbox for calculations the model should not do in its head |
| Alias | What production invokes. It points at a prepared version. `DRAFT` is not a production alias |
| `PrepareAgent` | Required after every instruction, schema, or knowledge-base change. Skipping it serves the old snapshot |
| Trace | `enableTrace` shows thought, tool, and observation. It is verbose and can contain sensitive tool payloads. Use it for debugging and for “show your steps” UIs |

Return of control is for tools that must run in the caller’s process: a private API, user-owned credentials, or a UI confirmation. The stream emits the API name and parameters and pauses. The caller posts the tool result on the next `InvokeAgent`. If nobody does, the session sits there. That looks like a hung agent.

`InvokeAgent` is `bedrock-agent-runtime`. Create, update, prepare, and alias calls are `bedrock-agent`.

## MCP

MCP is how a client discovers and calls tools. The model sees the tool name, description, and JSON schema. Vague descriptions cause the wrong tool to be chosen. That is a tool-selection failure, and the eval should measure it.

| Host | Tool |
|---|---|
| Lambda | Stateless, short, cheap (lookup, validate) |
| ECS or Fargate | Stateful connections, large dependencies, long work |
| AgentCore Gateway | REST you already have |

## Who runs the tool

| | Converse tool use | Classic agent | AgentCore |
|---|---|---|---|
| Who executes | Your code | Bedrock, unless return of control | Your code on Runtime, or the managed loop on Harness |
| Schema | `toolSpec` JSON schema | OpenAPI action group | Tool schema the runtime or gateway publishes |
| Retrieval | You call `Retrieve` | Associated for you | A tool you attach |
| Ops | You own the loop | Prepare, alias, trace | Runtime, Gateway, Memory, evals |

Converse tool turn:

1. Send `toolConfig` and the user message.
2. The response contains `toolUse` (`toolUseId`, `name`, `input`).
3. Your code runs the tool. The model does not call Lambda by itself on Converse.
4. The next message is `user` with `toolResult` for that `toolUseId`.
5. The model answers.

Treat tool arguments as untrusted. Validate types and ranges before the tool touches a database or a payment API. IAM on the tool role is the blast radius of a prompt injection. Specialists do not each receive every tool.

## Loop control

| Risk | Control |
|---|---|
| Runs forever | A step cap, a wall-clock timeout, a tool timeout, and a definition of done |
| Wrong account or data | IAM on the tool role, resource boundaries |
| Cascading failure | A circuit breaker around the tool and around the model |
| High-stakes side effect | A person, in front of the side effect |
| Several models | A router: classification on a small model, reasoning on a large one. An ensemble only when you need votes |

“The model will know when to stop” is not a control. Log tool name, latency, and error per step so a loop shows up as a metric and a trace.

Step Functions shape when you want the path drawn:

```text
Invoke model
  → Choice (tool requested, done, or clarify)
  → Lambda tool
  → append a trimmed observation
  → Invoke model
  → until stop or max steps
```

Human approval is a state that waits on a task token. Give that state a timeout. Notify with SNS. The agent may draft. A person sends.

## Memory

| Kind | Store | Keep it small |
|---|---|---|
| This session’s turns | Agent session or DynamoDB | Summarize. Replaying a long transcript fills the window |
| Summary | Agent memory or a summary prompt | Keep numbers and decisions the user still needs |
| Long-term user facts | AgentCore Memory or your database | Store what you would store for any user profile. Filter PII you do not need |
| Tool output | Trim in the tool | A multi-megabyte payload does not go into the next prompt |

## Patterns

| Name | Meaning |
|---|---|
| ReAct | Thought, action, observation |
| Plan-and-execute | Draft a plan, then run the steps. Easier to show a person |
| Router | Classify, then one specialist |
| Cascade | Try a cheap model or a tool, escalate when confidence is low |
| Reflection | A second prompt critiques the first. It costs tokens. The eval has to show it helps |
| Orchestrator-workers | A supervisor fans out and aggregates |

Add reflection or an extra agent when the eval shows a single prompt misses the quality bar. A single prompt that already passes stays a single prompt.
