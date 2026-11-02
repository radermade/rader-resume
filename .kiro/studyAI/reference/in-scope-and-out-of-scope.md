# In scope and out of scope

From the AIP-C01 exam guide (2026). Non-exhaustive on both sides. A service on the out-of-scope list is a distractor even when you know it well.

Job tasks that are **out of scope** even though the words appear nearby:

- Model development and training from scratch
- Advanced ML technique (custom losses, feature-store science, deep training theory)
- Data engineering and feature engineering as the job itself

You still **select** fine-tuning vs RAG, and you still **deploy** a model through Model Registry. You do not derive a training loop.

## In scope

**Analytics.** Athena, EMR, Glue, Kinesis, OpenSearch Service, QuickSight, MSK.

**Application integration.** AppFlow, AppConfig, EventBridge, SNS, SQS, Step Functions.

**Compute.** App Runner, EC2, Lambda, Lambda@Edge, Outposts, Wavelength.

**Containers.** ECR, ECS, EKS, Fargate.

**Customer engagement.** Amazon Connect.

**Database.** Aurora, DocumentDB, DynamoDB, DynamoDB Streams, ElastiCache, Neptune, RDS.

**Developer tools.** Amplify, CDK, CLI, CloudFormation, CodeArtifact, CodeBuild, CodeDeploy, CodePipeline, Kiro, SDKs, X-Ray.

**Machine learning.** Augmented AI, Bedrock, Bedrock AgentCore, Bedrock Knowledge Bases, Bedrock Prompt Management, Bedrock Prompt Flows, Comprehend, Kendra, Lex, Q Business, Q Business Apps, Q Developer, Amazon Quick, Rekognition, SageMaker AI, Clarify, Data Wrangler, Ground Truth, JumpStart, Model Monitor, Model Registry, Neo, Processing, Unified Studio, Textract, Titan, Transcribe.

**Management and governance.** Auto Scaling, Chatbot, CloudTrail, CloudWatch, CloudWatch Logs, CloudWatch Synthetics, Cost Anomaly Detection, Cost Explorer, Managed Grafana, Service Catalog, Systems Manager, Well-Architected Tool.

**Migration and transfer.** DataSync, Transfer Family.

**Networking.** API Gateway, AppSync, CloudFront, ELB, Global Accelerator, PrivateLink, Route 53, VPC.

**Security.** Cognito, Encryption SDK, IAM, IAM Access Analyzer, IAM Identity Center, KMS, Macie, Secrets Manager, WAF.

**Storage.** EBS, EFS, S3, S3 Intelligent-Tiering, S3 Lifecycle, S3 Cross-Region Replication.

## Out of scope

Do not build the answer on these.

| Area | Services |
|---|---|
| Application integration | Amazon MQ |
| Analytics | Clean Rooms, Data Exchange, DataZone, FinSpace |
| Blockchain | Managed Blockchain |
| Business apps | Alexa for Business, Chime, Wickr, WorkDocs, WorkMail |
| Cost | **Budgets, Cost and Usage Report, Reserved Instance reports, Savings Plans** |
| Compute | Batch, EC2 Image Builder, ECS Anywhere, EKS Anywhere, Elastic Beanstalk, Lightsail, Local Zones, Serverless Application Repository |
| Containers | App2Container, Copilot, ROSA |
| Customer engagement | **Amazon SES** |
| Database | Keyspaces, QLDB, **Redshift**, Timestream |
| Developer tools | Cloud9, CloudShell, **CodeGuru**, CodeStar, Corretto |
| End user | AppStream, WorkSpaces and the rest of that family |
| Front end | Device Farm, Location Service, Pinpoint |
| Games | GameLift, Lumberyard |
| IoT | The IoT family |
| Governance | Console mobile app, Health Dashboard, License Manager, Proton, **Trusted Advisor** |
| Machine learning | DeepComposer, DeepRacer, DevOps Guru, Forecast, Fraud Detector, HealthLake, Lookout family, Monitron, Panorama |
| Media | Elastic Transcoder, Elemental family, IVS, Kinesis Video Streams, Nimble Studio |
| Migration | Application Discovery, Application Migration Service, CloudEndure, Migration Hub, Snow Family |
| Networking | App Mesh, Cloud Map, **Direct Connect**, Private 5G, **Transit Gateway**, **VPN** |
| Quantum / robotics / satellite | Braket, RoboMaker, Ground Station |

## Borderline calls that show up as traps

| Stem feels like | In-scope choice | Out-of-scope trap |
|---|---|---|
| Notify operators | SNS, Chatbot | SES |
| Queue | SQS | MQ |
| Warehouse SQL | Athena on S3, Aurora | Redshift |
| Cost control | Cost Explorer, Cost Anomaly Detection, tags | Budgets, Savings Plans |
| Code assistant | Q Developer | CodeGuru |
| Host a web service | Lambda, ECS, App Runner | Elastic Beanstalk, Lightsail |
| Big batch of CPU jobs | Lambda, ECS, Step Functions, Bedrock batch | AWS Batch |
| Private connectivity to Bedrock | PrivateLink, VPC endpoints | VPN, Direct Connect, Transit Gateway as the answer |
| “Advisor” that finds waste | CloudWatch metrics, Cost Anomaly Detection | Trusted Advisor |
| Predict sales with classical ML | Not this exam’s job | Forecast |

Hybrid networking can exist in a real company. If the question’s *decision* is Direct Connect versus a Bedrock design, the genAI decision is still PrivateLink, residency, and what data moves. Do not select Direct Connect as the thing that makes the model safe.
