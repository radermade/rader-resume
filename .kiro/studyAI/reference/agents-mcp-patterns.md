# Agents, MCP, and orchestration patterns

## When it is an agent

The model must **choose** an action based on intermediate results, or call a tool that has a side effect. Otherwise use a prompt or a fixed Step Functions graph. Fixed graphs are easier to audit. Prefer them when the steps are known.

## Classic Bedrock Agents

Still the vocabulary of many questions.

| Piece | Rule |
|---|---|
| Instructions | System behavior of the agent |
| Foundation model | One primary model |
| Action group | OpenAPI 3 schema. Lambda fulfillment **or** return of control |
| Knowledge base | Retrieval tool with its own instructions |
| Guardrail | Id and numbered version, or it is not on |
| Memory | Session summary across turns |
| Code interpreter | Sandbox for calculations the model should not do in its head |
| Alias | What production invokes. Points at a version |
| PrepareAgent | Required after changes. Creates the version you can alias |
| Trace | `enableTrace` to see thought, tool, observation |
| Collaboration | Supervisor delegates to collaborator agents |

Return of control: use it when the tool must run in the caller’s process (private API, user-owned credentials, a UI confirmation). If the caller does not post the tool result back, the session stalls. That looks like a “hung agent.”

Parameter validation belongs in the Lambda. The model will send the wrong type eventually. IAM on that Lambda is the blast radius of a prompt injection.

Multi-agent: a supervisor for “which specialist,” specialists with narrow tools. AWS **Agent Squad** is the same topology as a library: classifier + agents. Do not give every specialist every tool.

## AgentCore

| Component | Use |
|---|---|
| Harness | New managed loop. Model, tools, memory, and instructions as config. Not classic action groups |
| Runtime | You bring the agent code. Pick HTTP or MCP. Session id required (long enough to be a real session key; the platform rejects short ids) |
| Gateway | Existing REST → MCP tools, so you do not write a tool server |
| Memory | Short-term session and longer-term facts. Not a reason to append the raw transcript forever |
| Evaluations | Task-level tests against the agent |
| Identity | Agent credentials to downstream tools. Still least privilege |

Classic → Harness migration is not behavior-identical. Return-of-control and the classic guardrail attachment are the pieces exams and migrations both stumble on. If the stem says guardrail on the agent resource, it is describing classic agents or an explicit guardrail on the model calls.

## MCP

Tools exposed in a standard way so any MCP client (the agent) can list and call them.

| Host | Tool type |
|---|---|
| Lambda | Stateless, short, cheap |
| ECS | Stateful connections, large dependencies, long work |
| AgentCore Gateway | REST you already have |

The model sees tool name, description, and JSON schema. Vague descriptions cause wrong tool choices. That is an eval failure (tool selection), not a model-capacity failure.

## Step Functions as the loop

Use states when you want the path in a diagram:

```text
Invoke model
  → Choice (tool requested? done? clarify?)
  → Lambda tool
  → append observation
  → Invoke model
  → … until stop or max steps
```

Human approval is a state that waits on a task token. Timeout that state. Notify with SNS. This is the pattern for “an agent may draft, a person must send.”

Circuit breaker: a state that counts failures in DynamoDB or in the execution and stops calling Bedrock or the tool. Graceful degradation is a branch to a smaller model or a static reply.

## Converse tools vs agents

| | Converse tool use | Classic agent |
|---|---|---|
| Who runs the tool | Your code | Bedrock, unless return of control |
| Schema | `toolSpec` JSON schema | OpenAPI action group |
| KB | You call Retrieve | Associated for you |
| Control | Full | Instructions + step limits |
| Ops | You own the loop | Prepare, alias, trace |

## Memory

| Kind | Store | Mistake |
|---|---|---|
| This session’s turns | Agent session or DynamoDB | Replaying 100 turns into the window |
| Summary | Agent memory feature or a summary prompt | Summarizing away a number the user needs |
| Long-term user facts | AgentCore Memory or your DB | Storing raw PII you could have filtered |
| Ephemeral tool output | Trim before the next turn | Pasting a 2 MB API payload into the prompt |

## Stopping

Every production agent needs a step cap, a wall-clock timeout, a tool timeout, and a definition of done. “The model will know when to stop” is not a control. Log tool name, latency, and error per step so a loop is a metric (Domain 4) and a trace (Domain 5).

## Human in the loop

| Need | Mechanism |
|---|---|
| Approve a side effect | Step Functions callback, or return of control to a UI |
| Review a completion before the customer sees it | Augmented AI |
| Score outputs for a dataset | Ground Truth |
| Edit a prompt before it is published | Prompt Management approval |

## Patterns named in stems

| Name | Meaning |
|---|---|
| ReAct | Thought, action, observation |
| Plan-and-execute | Draft a plan, then run steps. Easier to show a human |
| Router | Classify, then one specialist. Agent Squad |
| Cascade | Try a cheap model or a tool, escalate if confidence is low |
| Reflection | Second prompt critiques the first. Costs tokens. Eval should prove it helps |
| Orchestrator-workers | Supervisor fans out and aggregates |

Reflection and extra agents are wrong answers when the stem says least cost or least latency and a single prompt already meets the quality bar.
