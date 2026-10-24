---
inclusion: always
---

# AWS Serverless Standards

Scope: Lambda, Step Functions, and the supporting services for this project.
Target is a near-zero-cost, no-always-on-compute footprint.

## Ground rules

- Infrastructure is code. No console edits, ever. If it was clicked, it gets
  reverse-engineered into IaC or deleted.
- Least privilege per function. One role per function, scoped to the specific
  resources and actions it needs. No `Resource: "*"` outside of genuinely
  global actions.
- Everything tagged: `project`, `stage`, `component`, `managed-by`.
- Names include the stage. Nothing shared between stages except the hosted zone.
- No VPC unless a resource genuinely requires it. No NAT gateway.

## Lambda

- Runtime `provided.al2023`, architecture `arm64`.
- Set memory and timeout explicitly per function. Default timeouts are wrong in
  both directions: too long for a web handler, too short for a pipeline task.
- Set CloudWatch log retention explicitly (14 days dev, 30 days prod). The
  default is never-expire and it quietly accrues cost.
- Configuration via environment variables. Secrets via Secrets Manager or SSM
  Parameter Store, fetched at init, never logged, never in IaC literals.
- Public HTTP entry point: Lambda Function URL behind CloudFront with OAC.
  Reach for API Gateway only when you need something it actually provides
  (custom authorizers, usage plans, request validation, WebSockets) and record
  why.
- No unauthenticated mutating endpoints. Read paths may be public; anything
  that writes requires auth, enforced at the edge and re-checked in the
  handler.
- Alias-based deploys. CloudFront and Step Functions point at an alias, not
  `$LATEST`.

## Step Functions

- Use a state machine when there is orchestration, retry across boundaries,
  fan-out, or a workflow a human may need to inspect. Do not use one as glue
  for a single request/response.
- Standard workflows for publishing and scheduled syncs: execution history and
  long retries matter more than per-transition cost. Express only for
  high-volume, short, idempotent paths.
- Prefer direct SDK integrations over a Lambda that only forwards a call. Every
  Lambda in a workflow should contain logic worth running.
- Every Task state has an explicit `Retry` (with backoff and jitter) and a
  `Catch` routing to a failure state that records a diagnostic.
- Respect the 256 KB payload limit by design, not by luck. Pass S3 pointers for
  content; keep state payloads to identifiers and metadata.
- Use JSONata expressions and workflow Variables for data flow rather than
  deep `ResultPath` manipulation. Keep the data shape obvious from reading the
  definition.
- `Map` for bounded fan-out, Distributed Map for large or S3-driven fan-out.
  Set `ToleratedFailurePercentage` deliberately for enrichment work where
  partial success is acceptable.
- Executions are named deterministically from the triggering commit SHA where
  possible, so replays are visible and accidental double-triggers collide
  instead of duplicating work.
- Enable logging (`ALL` in dev, `ERROR` in prod) and X-Ray tracing.

## Interactive workflows

A visitor-facing stepped flow is a Standard workflow pausing on
`.waitForTaskToken`. The service holds the position in the flow; it does not
hold a read model.

- Never read execution state to render a page. `DescribeExecution` returns
  status and input, with output only after completion, so mid-flight data is
  invisible. `GetExecutionHistory` is paginated and rate-limited, Standard
  history is capped at 25k events with 90 day retention, and neither API is
  built for per-page-load traffic.
- Instead, push. The task Lambda that pauses the flow writes a projection of
  the current step to the session store, then blocks on its token. Reads serve
  from the projection and make zero Step Functions calls.
- This is CQRS in miniature: the state machine is the write side and holds
  authoritative position, the session record is the read side and holds only
  what the UI needs to render.
- Browsers never talk to Step Functions. There are no AWS credentials in a
  browser, ever. Every interaction goes through an authenticated endpoint that
  holds the token server-side.
- A Standard workflow parked on a token costs nothing while it waits and will
  wait up to a year, so an abandoned visitor flow is not a cost problem. Give
  the flow a sensible overall timeout anyway so executions don't accumulate.
- For a short step that computes and returns within the request, an Express
  workflow via `start_sync_execution` is the lighter option. Five minute
  ceiling, no task tokens.

## Supporting services

- S3: versioning on for content and artifact buckets, public access blocked,
  SSE enabled, lifecycle rules for old versions and incomplete uploads.
- CloudFront: OAC for S3 origins, separate cache policies for immutable assets
  (long TTL, hashed filenames) and HTML/API (short TTL, ETag-driven). Behavior
  precedence is load-bearing: specific path patterns like `/api/*` and
  `/flow/*` must be ordered ahead of any catch-all, and anything that must not
  be cached needs an explicit no-store policy rather than a short TTL.
- EventBridge Scheduler for scheduled triggers. Schedules live in IaC with the
  state machine they invoke.
- DynamoDB holds flow sessions and nothing else: on-demand billing, single
  table, partitioned on session ID, TTL on every item so abandoned flows
  evaporate. The item carries the execution ARN, the task token, the step
  projection, and a status. Adding any other access pattern to this table
  requires documenting it first.
- Bedrock for model calls, invoked only from task Lambdas inside a workflow,
  never from a request-path handler. Model latency is measured in seconds and
  throttling is routine. Rules for schema-constrained output, prompt
  versioning, and spend ceilings live in
  `.kiro/steering/ai-integration-standards.md`.
- ACM certificate for CloudFront in `us-east-1`; everything else in the primary
  region.

## Observability and cost

- Structured JSON logs with correlation IDs: request ID for web, execution name
  plus commit SHA for pipelines.
- Alarms that matter: state machine `ExecutionsFailed`, function `Errors` and
  `Throttles`, CloudFront 5xx rate. Route them somewhere you will actually see.
- A budget alarm exists before the first deploy.
- Review cost by looking at log volume first. On a project this size, CloudWatch
  Logs is the most likely line item to surprise you.
