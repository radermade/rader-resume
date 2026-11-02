# Domain 2 — Implementation and integration

**26% of scored content.** Tasks 2.1–2.5. How the model is called, hosted, wired into the enterprise, and wrapped in tools. Domain 1 chose the pattern. This domain builds it.

## 2.1 Agents and tools

An agent is a loop: model plans, calls a tool, reads the observation, repeats, stops. If there is no tool and no branch, it is a prompt, not an agent.

### What the exam still names

| Option | What it is | Stem language |
|---|---|---|
| Amazon Bedrock Agents (classic) | Managed loop: instructions, model, action groups, knowledge bases, guardrails, memory, code interpreter | Action group, agent alias, `PrepareAgent`, OpenAPI schema, return of control |
| Bedrock AgentCore Harness | Config-based managed loop on AgentCore. No container | “Managed agent loop,” model + tools + memory as config |
| AgentCore Runtime | Hosts an agent loop **you** wrote | “Deploy my agent,” HTTP or MCP protocol |
| AgentCore Gateway | Turns REST APIs into MCP tools | “Expose existing APIs as tools” |
| Strands Agents | SDK for building agent behavior and tools | “Strands API,” custom tool behavior |
| AWS Agent Squad | Multi-agent: a classifier routes to specialist agents | “Several specialized agents,” supervisor |
| Step Functions | Explicit state machine for ReAct, approvals, retries | Deterministic control, audit, human step |
| MCP server | Tool process the model calls through the protocol | Stateless → Lambda. Heavy → ECS |

Classic Agents are in maintenance for **new** customers. The exam guide still tests them. Answer with the product the stem describes.

### Classic agent pieces

- **Action group**: OpenAPI schema + Lambda, or **return of control** (your app runs the API and sends the result back).
- **Knowledge base** attached to the agent: retrieval is a built-in tool.
- **Guardrail** on the agent: id **and** version, or it silently does nothing. Use a numbered version, not `DRAFT`.
- **Memory**: session summary so the next turn is cheaper than replaying the transcript.
- **Alias**: points at a version. Apps call the alias. `DRAFT` is not a production alias.
- **PrepareAgent** after every instruction, schema, or KB change. Skipping it serves the old snapshot.
- **Supervisor / collaboration**: one agent delegates to specialists. Agent Squad is the same idea outside classic Agents.

Treat tool arguments as untrusted. Validate types and ranges in Lambda before the tool touches a database or a payment API.

### Control the loop

| Risk | Control |
|---|---|
| Runs forever | Max steps, Step Functions timeout, stopping condition |
| Calls the wrong account | IAM on the tool role, resource boundaries |
| Cascading failure | Circuit breaker around the tool and around the model |
| High-stakes action | Human approval: Step Functions callback (task token) or API Gateway feedback |
| Several models | Router sends classification to a small model and reasoning to a large one. Ensemble only if you need votes, not by default |

### ReAct vs chain-of-thought

- Chain-of-thought: the model writes steps, no external tool.
- ReAct: the model writes a thought, **calls a tool**, reads the observation, repeats. Implement in an agent runtime or as Step Functions states (invoke model → choice → Lambda tool → invoke model).

### MCP

Model Context Protocol is how a client discovers and calls tools. Exam split:

- **Lambda** MCP server: stateless, short tools (lookup, validate).
- **ECS** MCP server: long-running, heavy, or connection-pooled tools.
- AgentCore Gateway: existing REST, no custom MCP server to write.

**Pick this when:** “OpenAPI + Lambda + Bedrock agent” → action group. “I already have a Python agent” → AgentCore Runtime. “REST catalog as tools” → Gateway. “route billing questions to one bot and HR to another” → Agent Squad or a supervisor. “legal must approve the email” → Step Functions human step, not a bigger prompt.

## 2.2 Deployment

| Shape | Fit |
|---|---|
| Lambda calls Bedrock on demand | Spiky, low ops, seconds-level work, payload limits |
| Bedrock provisioned throughput | Steady high volume, or a custom model that cannot use on-demand |
| SageMaker endpoint | Model you trained or tuned outside Bedrock, GPU control, VPC-only container |
| Container on ECS/EKS | Large memory, custom server, GPU, token batching you control |
| API cascade | Small model answers easy calls; hard calls go up-stack |
| Smaller task-specific model | Classification, extraction. Do not pay Opus prices for a label |

LLM hosting is not classic ML hosting. Watch **GPU memory**, **token throughput** (not requests alone), and **cold start** when the container loads weights. SageMaker Neo is for compiling classic models to run faster; it is not how you speed up a Bedrock API call.

Hybrid: Bedrock for the general model, SageMaker for the domain model you cannot get on Bedrock.

**Pick this when:** “no servers, variable traffic” → Lambda + on-demand Bedrock. “reserved capacity, custom model” → provisioned throughput. “weights we trained” → SageMaker + Model Registry. “most questions are easy” → cascade, not one large model for everything.

## 2.3 Enterprise integration

### Coupling

| Style | Use |
|---|---|
| Sync API | User is waiting. API Gateway → Lambda → Bedrock |
| Event-driven | Producer must not wait. EventBridge → consumer. Loose coupling, retries, DLQ |
| Queue | Buffer and smooth spikes. SQS → Lambda. FIFO only if order matters |
| Webhook | External system calls you. Verify signatures. Lambda handler |
| Batch | Overnight. No API Gateway in the hot path |
| Data sync | AppFlow for SaaS, DataSync or Transfer Family for files, Glue for lakes |

### Identity

- Users: Cognito or IAM Identity Center. Do not share one IAM user access key in the mobile app.
- Service: IAM **role**. Least privilege on `bedrock:InvokeModel` and tool APIs.
- Federation: enterprise IdP → IAM role → Bedrock. RBAC on which model and which knowledge base.
- Force a guardrail with the IAM condition `bedrock:GuardrailIdentifier` so a caller cannot omit it.

### Data stays put

| Constraint | Pattern |
|---|---|
| On-prem data cannot move | Outposts, or process on-prem and send only the prompt you are allowed to send |
| Ultra-low latency at the edge | Wavelength (carrier edge), Lambda@Edge / CloudFront only for light request shaping, not for a big model |
| EU residency | `eu.` inference profile, EU Regions, no `global.` profile |
| Private network | VPC interface endpoints (PrivateLink) for `bedrock-runtime`. No public internet path |

### CI/CD and a GenAI gateway

Pipeline: CodePipeline → CodeBuild (unit, prompt regression, security scan) → deploy → canary → rollback (CodeDeploy or alias flip).

A **gateway** is one front door: auth, request validation, model routing, token limits, guardrail injection, logging, WAF. Apps do not call Bedrock with scattered credentials. API Gateway or a small service is the usual implementation. AppConfig holds the route table.

**Pick this when:** “legacy must not call Bedrock directly” → gateway. “SAP emits an event, summary can lag” → EventBridge, not a synchronous plugin inside SAP. “on-prem database” → Outposts or a private pull, not a public S3 upload of the raw table.

## 2.4 Calling the FM API

### Sync, async, stream

| Mode | API / service | When |
|---|---|---|
| Sync | `Converse` on `bedrock-runtime` | Backend job wants the full answer |
| Stream | `ConverseStream` (SDK; CLI does not stream) | Chat UI. Tokens as they generate |
| Stream to browser | API Gateway WebSocket, Lambda response streaming, or chunked HTTP. SSE if the stack supports it | User should see text immediately |
| Async | SQS + Lambda, or Step Functions | Work longer than the API timeout, or fan-out |
| Custom client | API Gateway validates the body, then invokes | External consumers must not hold Bedrock IAM |

Set **`maxTokens` on every call**. Unset means “reserve the model max,” which throttles you even at low request rates.

### Resilience on the client

- AWS SDK **adaptive** retry: throttling, timeout, 5xx. Not validation or access denied.
- API Gateway throttles **your** clients (usage plans). That is separate from Bedrock quotas.
- Fallback model or cached answer when the primary circuit is open.
- X-Ray across API Gateway → Lambda → Bedrock so a slow prompt is a segment, not a mystery.

### Routing

| Kind | Where it lives |
|---|---|
| Static | Config map: route `/summarize` → Haiku |
| Content-based | Step Functions or gateway inspects the payload, picks a specialist model |
| Metric-based | Latency or error rate shifts traffic |
| Transform | API Gateway mapping templates adapt an old client payload to Converse |

**Pick this when:** “tokens appear as they are generated” → ConverseStream plus a streaming edge, not a bigger timeout on a buffered Lambda. “spikes must not fall on the floor” → SQS. “ThrottlingException at low QPS” → `maxTokens` and quotas before you add provisioned throughput.

## 2.5 App patterns and developer tools

| Need | Tool |
|---|---|
| HTTP API for a genAI backend, streaming, timeouts, API keys | API Gateway |
| UI that calls that API | Amplify (hosting, auth, declarative client) |
| Contract-first | OpenAPI. Action groups and API Gateway both consume schemas |
| No-code prompt chain | Bedrock Prompt Flows |
| CRM field, ticket summary | Lambda inside the existing app, or EventBridge |
| Document-heavy extraction | Textract and/or Bedrock Data Automation, orchestrated by Step Functions |
| Write or refactor the integration code | Amazon Q Developer |
| Enterprise users asking questions of company data | Amazon Q Business (not Q Developer) |
| Trace a bad completion | CloudWatch Logs Insights on prompts and responses, X-Ray on the API call, Q Developer to spot the error pattern |

Q Developer is the **builder** assistant. Q Business is the **employee** assistant over enterprise data. Swapping them is a common distractor.

Advanced apps on this exam are still the same three shapes: Strands or Agent Squad for multi-agent, Step Functions for an explicit graph, Prompt Flows or prompt chaining for a fixed pipeline.

**Pick this when:** “developers need code help in the IDE” → Q Developer. “employees search internal wiki in natural language, with permissions” → Q Business or a KB with metadata filters. “business user edits the chain” → Prompt Flows. “we need a reviewed pipeline in git” → CodePipeline, not Prompt Flows.

## Domain 2 traps

- An agent for a single completion with no tool.
- Forgetting `PrepareAgent` or calling the `DRAFT` alias in production.
- Action group without validation of model-supplied parameters.
- Lambda buffered response when the stem says streaming.
- `bedrock-agent-runtime` used to invoke a base model. Model calls go to `bedrock-runtime`. Agent and KB data calls go to `bedrock-agent-runtime`.
- Amazon MQ, Elastic Beanstalk, or SES as the integration. They are out of scope. Use SQS, EventBridge, Lambda, API Gateway.
- CodeGuru for “AI code help.” The in-scope assistant is Q Developer. CodeGuru is out of scope.
