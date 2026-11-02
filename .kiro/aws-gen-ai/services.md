---
inclusion: auto
name: aws-gen-ai-services
description: "Use when choosing which AWS service owns a job in a generative AI system: Bedrock, Knowledge Bases, AgentCore, SageMaker, Kendra, Q Developer, Q Business, vector stores, queues, identity, logging, and cost."
---

# Which service owns the job

Name the job, then the service. If the design does not have that job, leave the service out.

## Model, retrieval, and agents

| Service | Owns |
|---|---|
| Amazon Bedrock | Hosted foundation-model inference, customization of supported models, guardrails, batch, provisioned throughput |
| Bedrock Knowledge Bases | Ingest, chunk, embed, retrieve, and optional generate-with-citations |
| Bedrock Prompt Management | Shared, versioned, parameterized prompts and the approval of a prompt |
| Bedrock Prompt Flows | A visual prompt chain for builders who should not ship that chain as code |
| Bedrock AgentCore | Runtime for a loop you wrote, Gateway for REST as MCP tools, Harness for a managed loop, memory, evals |
| Bedrock Agents (classic) | An existing agent built from action groups, aliases, and `PrepareAgent` |
| Amazon Titan | Amazon’s models. Text Embeddings V2 is the default embedding model |
| Strands Agents | An SDK for agent behavior and tools you write |
| AWS Agent Squad | A classifier that routes to specialist agents |
| Amazon Kendra | Managed enterprise search with connectors and ACLs, when you are not running the chunk-and-embed pipeline |
| Amazon Lex | An intent-and-slot bot, including a front end that hands the hard turn to Bedrock |
| Amazon Q Developer | The builder: code, refactors, API help, error explanation |
| Amazon Q Business | The employee: questions over connected enterprise data, with permissions |
| SageMaker AI endpoints | A model you trained, or one Bedrock does not host. GPU and VPC control |
| SageMaker JumpStart | A starting point from a model hub, then tune or deploy |
| SageMaker Model Registry | Version and approve model packages. The rollback target |
| SageMaker Processing | Batch data-preparation jobs |
| SageMaker Data Wrangler | Interactive tabular preparation |
| SageMaker Ground Truth | Human labeling for an eval or training set |
| SageMaker Model Monitor | Drift on a classic deployed model |
| SageMaker Clarify | Bias and feature attribution on classic models |
| SageMaker Neo | Compile a classic model for faster inference |
| SageMaker Unified Studio | The studio workspace for building and governing |
| Amazon Augmented AI | A person reviews a live output before it hits a customer or a system of record |

## Understand the input

| Service | Owns |
|---|---|
| Amazon Textract | OCR, forms, and tables out of documents before chunking or prompting |
| Amazon Transcribe | Audio to text before a text model |
| Amazon Rekognition | Image labels and moderation in a pipeline |
| Amazon Comprehend | Entities, sentiment, key phrases, and PII on text you submit |
| Amazon Macie | Sensitive data already sitting in S3 |

A multimodal Bedrock call owns “understand this image and this text together” when that is the requirement. A Transcribe-plus-Textract pipeline owns the case where you need the intermediate text.

## Move the request

| Service | Owns |
|---|---|
| Lambda | Glue between the API, the queue, and Bedrock. Short transforms and stateless tool handlers |
| Lambda@Edge | Light request logic at CloudFront. The model call stays in the Region |
| API Gateway | The front door: auth, validation, throttles, WebSocket or streaming HTTP |
| AppSync | Clients that are already GraphQL |
| Step Functions | A durable multi-step path: a drawn loop, a human approval, retries, fan-out |
| SQS | A buffer and a DLQ. FIFO when order is required |
| SNS | Fan-out notifications, including “a person needs to approve this” |
| EventBridge | Events between systems, and a schedule for reindex |
| AppConfig | Model id, prompt id, and feature flags without a redeploy. Parameter Store is the simpler cousin for a single id |
| AppFlow | SaaS data into the corpus |
| ECS / Fargate | A long-running MCP server, a custom model server, a heavy tool process |
| EKS | The same, when the platform is already Kubernetes |
| ECR | Images for those tasks and for SageMaker |
| App Runner | A simple container HTTP service |
| Outposts | AWS services on premises so the data can stay |
| Wavelength | Carrier-edge latency |
| Amazon Connect | The contact-center channel in front of the bot |

## Store it

| Service | Owns |
|---|---|
| S3 | Corpus, invocation logs, prompt files, artifacts. Lifecycle expires sensitive data |
| S3 Vectors | A managed vector bucket for a Knowledge Base |
| OpenSearch | Vector plus keyword plus filters. The usual store you operate yourself |
| Aurora or RDS PostgreSQL | pgvector when Postgres is already there |
| Neptune | Graph retrieval |
| DynamoDB | Session, conversation summary, request metadata. Streams to trigger an update |
| ElastiCache | An exact or semantic response cache, and hot session state |
| DocumentDB | A document store the system already uses |
| Glue | Catalog, crawlers, ETL, Data Quality rules, lineage |
| Athena | SQL on logs and eval results in S3 |
| Kinesis | A stream into ingestion |
| MSK | Kafka that is already in the architecture |

## Secure and see it

| Service | Owns |
|---|---|
| IAM | Authorization on every API. A condition to force a guardrail |
| IAM Identity Center | Workforce federation |
| Cognito | Application users |
| KMS | Customer-managed keys on logs, buckets, guardrails, and the knowledge base |
| Secrets Manager | Vector-store passwords and third-party tokens |
| VPC interface endpoints | A private path to Bedrock |
| WAF | Edge filtering on API Gateway or CloudFront |
| CloudTrail | Who called which API. Data events for invoke and retrieve |
| CloudWatch | Metrics, alarms, and Logs Insights |
| CloudWatch Synthetics | A canary against the public endpoint |
| X-Ray | API to Lambda to model to tool |
| Lake Formation | Table and column permissions on the lake that feeds the corpus |
| Encryption SDK | The application encrypts before the bytes reach S3 |
| IAM Access Analyzer | A review of policies that are too broad |
| Service Catalog | An approved generative-AI product other accounts launch |
| Cost Explorer | Who spent what, by service, tag, and time |
| Cost Anomaly Detection | A spike alert |
| Managed Grafana | An operational dashboard across metrics |
| QuickSight | A business report of quality, cost, and usage |

## Ship it

| Service | Owns |
|---|---|
| CodePipeline, CodeBuild, CodeDeploy | CI/CD with an eval gate and a rollback |
| CodeArtifact | Internal packages: a shared prompt SDK, tool libraries |
| CloudFormation or CDK | The stack |
| CLI and SDKs | How applications call Bedrock |
| Amplify | Front end and auth around the API |
| CloudFront | Public static content and cached answers that are not personalized |
| Route 53 | DNS and health checks in front of the gateway |
| Global Accelerator | An anycast path to regional endpoints |
| ELB | In front of ECS or SageMaker containers |
| DataSync or Transfer Family | Bulk files into the corpus |
| Auto Scaling | An endpoint or fleet that has a scalable dimension |

## Pairs that get swapped

| Job | Service |
|---|---|
| Help a developer write code | Q Developer |
| Let an employee ask about company data | Q Business |
| Find PII in text you submit | Comprehend |
| Find sensitive data already in S3 | Macie |
| Run your own retrieval pipeline | Knowledge Bases, usually on OpenSearch or a managed store |
| Hand someone a search product with connectors and ACLs | Kendra |
| Stay on Postgres | pgvector |
| Need hybrid search and rich filters | OpenSearch |
| Seconds, stateless | Lambda |
| Long-running or a pooled connection | ECS |
| A queue of work | SQS |
| An event and a routing rule | EventBridge |
| Steps you can draw | Step Functions |
| The model chooses the tool | An agent |
| Author a prompt chain | Prompt Flows |
| Gate a release | CodePipeline |
| A person reviews a live output | Augmented AI |
| A person labels a dataset | Ground Truth |
| See who spent the tokens | Cost Explorer, with inference-profile tags |
| Hear about a cost spike | Cost Anomaly Detection |
| One schema for every model | Converse |
| A provider-specific body | InvokeModel |
| Chunks for your own prompt | `Retrieve` |
| An answer and citations | `RetrieveAndGenerate` |
| What the text is allowed to say | A guardrail |
| Who is allowed to call | IAM |
| Change the model id safely | AppConfig |
| Keep data in a geography | A geo inference profile (`eu.`, `us.`, `apac.`) |
| Maximum burst, geography not guaranteed | A `global.` profile |
