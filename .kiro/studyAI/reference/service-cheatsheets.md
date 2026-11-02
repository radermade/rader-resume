# Service cheat sheets

One line for what the service is doing in a **genAI** design. If the stem does not need that job, do not pick the service.

## Machine learning

| Service | Wins when |
|---|---|
| Amazon Bedrock | Hosted FM inference, customization of supported models, guardrails, batch, provisioned throughput |
| Bedrock Knowledge Bases | RAG: ingest, chunk, embed, retrieve, optional generate with citations |
| Bedrock Prompt Management | Shared, versioned, parameterized prompts and approvals |
| Bedrock Prompt Flows | Visual prompt chains and branches for builders who should not ship code |
| Bedrock AgentCore | Runtime (your agent), Gateway (APIs as MCP tools), Harness (managed loop), memory, evals |
| Bedrock Agents (classic) | Action groups, aliases, PrepareAgent, built-in KB and guardrail. Still on the exam |
| Amazon Titan | Amazon’s models. Embeddings V2 is the default embed choice on the exam |
| Amazon Q Developer | Developers: code, refactors, API help, error explanation |
| Amazon Q Business | Employees: questions over connected enterprise data, with permissions |
| Amazon Comprehend | Entities, sentiment, key phrases, PII on a string you submit |
| Amazon Macie | Find sensitive data already in S3 |
| Amazon Kendra | Managed search with connectors and ACLs when you do not want to own embeddings |
| Amazon Lex | Intent and slot bots, including a front end that hands hard turns to Bedrock |
| Amazon Rekognition | Image labels and moderation in a pipeline |
| Amazon Textract | OCR, forms, tables out of documents before chunking or prompting |
| Amazon Transcribe | Audio to text before a text model |
| SageMaker AI endpoints | Deploy models you trained or that Bedrock does not host. GPU control |
| SageMaker JumpStart | Start from a model hub, then tune or deploy |
| SageMaker Model Registry | Version and approve model packages. Rollback target |
| SageMaker Processing | Batch data prep jobs |
| SageMaker Data Wrangler | Interactive tabular prep |
| SageMaker Ground Truth | Human labeling for eval or training sets |
| SageMaker Model Monitor | Drift on classic deployed models |
| SageMaker Clarify | Bias and feature attribution on classic models |
| SageMaker Neo | Compile classic models for faster inference. Not a Bedrock knob |
| SageMaker Unified Studio | The studio workspace the guide lists for building and governing |
| Amazon Augmented AI | Human review loop on live outputs |
| Amazon Quick | Listed in scope on the 2026 guide. Do not invent a design around it unless the stem names it. Confirm the current product page before test day |

## Integration and compute

| Service | Wins when |
|---|---|
| Lambda | Glue between API, queue, and Bedrock. Short transforms, tool handlers, MCP tools that are stateless |
| Lambda@Edge | Light request logic at CloudFront. Not the place to run a model |
| API Gateway | Front door: auth, validation, throttles, WebSocket or streaming HTTP |
| AppSync | GraphQL clients. Less common here unless the stem is already GraphQL |
| Step Functions | Durable multi-step: ReAct you can draw, human approval, retries, fan-out |
| SQS | Buffer, async inference, DLQ. FIFO only for strict order |
| SNS | Fan-out notifications |
| EventBridge | Events between systems, schedules for reindex, loose coupling |
| AppConfig | Model id, prompt id, feature flags without a redeploy |
| AppFlow | SaaS to AWS data movement into a corpus |
| ECS / Fargate | Long-running MCP servers, custom model servers, heavy tool processes |
| EKS | Same, when the platform is already Kubernetes |
| ECR | Images for those tasks and for SageMaker |
| EC2 | GPU box you manage. Lose to SageMaker or Bedrock unless they require the OS |
| App Runner | Simple container HTTP service |
| Outposts | AWS services on-prem so data can stay |
| Wavelength | Carrier-edge latency |

## Data

| Service | Wins when |
|---|---|
| S3 | Corpus, invocation logs, prompt repo, artifacts. Lifecycle to expire sensitive data. CRR if the stem requires another Region |
| S3 Vectors | Managed vector bucket for a KB |
| Glue | Catalog, crawlers, ETL, Data Quality rules, lineage |
| Athena | SQL on logs and eval results in S3 |
| EMR | Heavy custom processing. Rarely the least-overhead answer |
| Kinesis | Streaming events into an ingestion path |
| MSK | Kafka already in the architecture |
| OpenSearch Service | Vector + keyword + filters. The usual customer-managed store |
| Aurora / RDS PostgreSQL | pgvector when Postgres is already there |
| DynamoDB | Session state, conversation summary, request metadata, feature flags. Streams to trigger updates |
| DynamoDB Streams | Change data capture into the index |
| ElastiCache | Semantic or exact response cache, hot session |
| DocumentDB | Document store the stem already uses |
| Neptune | Graph plus graph RAG |

## Security and governance

| Service | Wins when |
|---|---|
| IAM | AuthZ for every API. Conditions to force a guardrail |
| IAM Identity Center | Workforce federation |
| Cognito | App users |
| KMS | CMK on logs, buckets, guardrails, KB |
| Secrets Manager | Vector DB keys, third-party tokens. Not in the prompt |
| VPC + PrivateLink | Private Bedrock endpoints |
| WAF | Edge filtering on API Gateway or CloudFront |
| CloudTrail | Who called which API. Data events for Invoke and Retrieve |
| CloudWatch | Metrics, alarms, Logs Insights on prompts |
| CloudWatch Synthetics | Canaries that hit the public endpoint |
| X-Ray | Trace API → Lambda → model → tool |
| Macie | S3 sensitive-data discovery |
| Lake Formation | Table and column permissions on the lake that feeds the corpus |
| Encryption SDK | Client-side encryption when the stem says the app encrypts before S3 |
| Access Analyzer | Find overly broad policies. A review tool, not a runtime filter |
| Systems Manager | Parameter Store is the simple cousin of AppConfig for a model id |
| Service Catalog | Publish an approved genAI product for accounts to launch |
| Well-Architected Tool | GenAI Lens review |
| Config | Not the star. AppConfig is the runtime config service |

## Developer and ops

| Service | Wins when |
|---|---|
| CodePipeline / CodeBuild / CodeDeploy | CI/CD with eval and rollback |
| CodeArtifact | Internal packages (shared prompt SDK, tool libs) |
| CloudFormation / CDK | IaC for the stack |
| CLI and SDKs | The way apps call Bedrock |
| Amplify | Front end and auth around the API |
| X-Ray | Already listed. Use it in troubleshooting stems |
| Kiro | Listed developer tool on the guide |
| Cost Explorer | Who spent what, by service, tag, time |
| Cost Anomaly Detection | Spike alerts. Prefer this over Budgets (Budgets is out of scope) |
| Auto Scaling | Endpoints and fleets that have a scalable dimension |
| Managed Grafana | Operational dashboards across metrics |
| QuickSight | Business reports of quality, cost, usage |
| CloudFront | Public static content and **non-personalized** cached answers |
| Route 53 | DNS and health checks in front of a gateway |
| Global Accelerator | Anycast path to regional endpoints |
| ELB | In front of ECS or SageMaker containers |
| DataSync / Transfer Family | Bulk file movement into the corpus |
| Amazon Connect | Contact center. The bot behind a voice or chat channel |

## Quick “versus”

| A | B | Rule |
|---|---|---|
| Q Developer | Q Business | Builder vs employee |
| Comprehend | Macie | Text API vs S3 discovery |
| Kendra | Knowledge Bases | Managed search product vs your RAG pipeline |
| OpenSearch | pgvector | Search features vs stay on Postgres |
| Lambda | ECS | Seconds and stateless vs long-running or GPU |
| SQS | EventBridge | Queue of work vs event fan-out and routing rules |
| Step Functions | Agent | Known graph vs model-chosen tools |
| Prompt Flows | CodePipeline | Authoring a chain vs shipping and gating a release |
| A2I | Ground Truth | Review live outputs vs label a dataset |
| Cost Explorer | Budgets | In scope vs out of scope |
| Converse | InvokeModel | Default vs provider-specific |
| Retrieve | RetrieveAndGenerate | Chunks vs answered and cited |
| Guardrail | IAM | Content policy vs who may call |
| AppConfig | redeploy | Change model id safely vs ship code |
| Cross-Region `eu.` | `global.` | Residency vs maximum burst |
