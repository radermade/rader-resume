# Phase 7 — Tasks

Status: outline

## Decide first

- [ ] Admin UI, or editing through commits only (roadmap open question 5)

## Operations

- [ ] Alarms: pipeline `ExecutionsFailed`, function errors and throttles,
      CloudFront 5xx
- [ ] Route alarms somewhere actually monitored
- [ ] Confirm log retention is explicit on every log group
- [ ] Cost review, with log volume checked first
- [ ] Bedrock spend ceiling confirmed in place

## Transparency

- [ ] Pipeline writes an execution summary object per run
- [ ] Transparency page reading those summaries, not `ListExecutions`
- [ ] Architecture decision notes as site content

## Hardening

- [ ] IAM review: per-function roles, no wildcards
- [ ] Restrict the Function URL now that CloudFront fronts it
- [ ] Integration tests against a deployed stage
- [ ] Root credential cleanup: Identity Center or IAM user with MFA, delete
      root access keys

## If an admin UI happens

- [ ] Authentication at the edge, re-checked in the handler
- [ ] Writes route through the pipeline, never around it
