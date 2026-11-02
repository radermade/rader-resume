---
inclusion: auto
name: aws-gen-ai-runtime
description: "Use when hosting, integrating, or shipping a generative AI workload on AWS: Lambda, API Gateway, streaming, queues, Step Functions, SageMaker endpoints, provisioned throughput, VPC, data residency, identity, and CI/CD."
---

# Runtime and integration

## Where the model runs

| Shape | Fit |
|---|---|
| Lambda calls Bedrock on demand | Spiky traffic, low operations, work that finishes in seconds, inside the payload limits |
| Bedrock provisioned throughput | Steady high utilization, or a custom model that cannot use on-demand |
| Reserved tier | Mission-critical capacity with an uptime target, on a 1- or 3-month commit |
| SageMaker endpoint | Weights you trained, GPU control, a VPC-only container |
| ECS, Fargate, or EKS | Long-running MCP servers, custom model servers, heavy tool processes. EKS when the platform is already Kubernetes |
| EC2 | A GPU box you must manage yourself. Prefer SageMaker or Bedrock when they can host it |
| App Runner | A simple container HTTP service |
| Outposts | The data has to stay on premises and still use AWS services there |

Watch GPU memory, token throughput, and cold start when a container loads weights. Request rate alone hides a few huge prompts. SageMaker Neo compiles classic models. It does not speed up a Bedrock API call.

Autoscale SageMaker and ECS on GPU memory and in-flight tokens.

Provisioned throughput wins when utilization is high enough that on-demand would cost more. A spiky proof of concept stays on demand.

## Sync, stream, async

| Mode | Use |
|---|---|
| `Converse` | The caller wants the full answer. A backend job |
| `ConverseStream` | A person is watching tokens. SDK only |
| Stream to the browser | API Gateway WebSocket, Lambda response streaming, or chunked HTTP. SSE when the stack supports it |
| SQS, then Lambda | Buffer spikes. FIFO only when order matters. A DLQ for poison messages |
| EventBridge | Something else happened and the summary can lag. Schedules for reindex. Loose coupling |
| Step Functions | Fan-out, retries across boundaries, a human, an audit of each step |
| Batch inference | Thousands of documents, nobody waiting, discounted async |
| Webhook | An external system calls you. Verify the signature in Lambda |

A buffered Lambda response makes a chat UI stare at a blank page. Streaming hides output time. It does not hide retrieval time. Precompute hides both for questions you already know.

API Gateway throttles your clients (usage plans). That quota is separate from Bedrock quotas. External consumers hit the gateway. They do not hold Bedrock credentials.

| Routing | Where it lives |
|---|---|
| Static | Config: `/summarize` maps to a small model |
| Content-based | The gateway or Step Functions inspects the payload |
| Metric-based | Latency or error rate shifts traffic |
| Transform | API Gateway mapping templates adapt an old client body to Converse |

## Identity and the front door

A gateway is the one front door: auth, validation, model routing, token limits, guardrail injection, logging, WAF. Applications do not call Bedrock with scattered credentials. AppConfig holds the route table.

- People: Cognito, or IAM Identity Center for the workforce.
- Services: an IAM role, least privilege on `bedrock:InvokeModel`, `bedrock:InvokeModelWithResponseStream` when streaming, and the tool APIs.
- Force the guardrail with `bedrock:GuardrailIdentifier`.

| Constraint | Pattern |
|---|---|
| Private network path to Bedrock | VPC interface endpoints for `bedrock-runtime` |
| Data must stay in the EU (or another geography) | That geo’s inference profile and Regions |
| On-prem data cannot move | Outposts, or process in place and send only the prompt you are allowed to send |
| Ultra-low latency at a carrier edge | Wavelength |
| Light request shaping at CloudFront | Lambda@Edge. The model call stays in the Region |

## Data movement into the corpus

| Source | Path |
|---|---|
| Objects and artifacts | S3. Lifecycle to expire sensitive data. Cross-Region replication when a second Region is a real requirement |
| SaaS | AppFlow |
| Bulk files | DataSync or Transfer Family |
| Lake tables | Glue. Athena for SQL on logs and eval results in S3 |
| Streams | Kinesis, or MSK when Kafka is already the bus |
| Row changes that must be re-embedded | DynamoDB Streams |

## CI/CD

CodePipeline → CodeBuild (unit tests, prompt regression, safety set, security scan) → deploy → canary → rollback by alias or CodeDeploy.

Before shifting an alias: replay the golden set, compare semantically (string equality is for labels, not paragraphs), measure unsupported claims and schema failures, run the safety set. Roll the alias back when the gate fails. Change one of temperature, chunking, or model per deploy so the diff has a cause.

Synthetic journeys catch multi-turn breaks that single-shot tests miss.

## Developer surfaces

| Need | Tool |
|---|---|
| HTTP API, streaming, timeouts, API keys | API Gateway |
| A UI and auth around that API | Amplify |
| Contract-first tools and routes | OpenAPI. Action groups and API Gateway both consume schemas |
| Write or refactor the integration | Amazon Q Developer |
| Employees asking questions over connected company data, with permissions | Amazon Q Business |
| Trace a bad completion | X-Ray from API Gateway through Lambda, the model, and the tool. Logs Insights on prompt version. Q Developer to read the error pattern |
| Contact center channel | Amazon Connect, with the bot behind it |
| Internal packages (shared prompt SDK, tool libraries) | CodeArtifact |

Q Developer is the builder assistant. Q Business is the employee assistant over enterprise data.
