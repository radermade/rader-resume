# Phase 5 — Tasks

Status: not started

## Decide first

- [ ] Which sources, and at what frequency
- [ ] Move PDF generation to Phase 4's render fan-out and drop it from here
- [ ] Whether `dev` runs on a schedule

## Build

- [ ] Derived-data types with provenance in `resume-core`
- [ ] Source config as content in the repo, not hardcoded
- [ ] `tasks/sync`: one source per invocation
- [ ] Normalize into derived types, no authored-field overwrites
- [ ] Diff against `content/` and skip no-op runs
- [ ] Bot PR creation with a readable diff and a body explaining provenance

## Workflow

- [ ] `Map` with deliberate `ToleratedFailurePercentage`
- [ ] Per-branch retry for flaky external APIs
- [ ] EventBridge Scheduler in IaC alongside the state machine

## Infrastructure

- [ ] External API credentials in Secrets Manager or Parameter Store
- [ ] A GitHub token scoped to opening pull requests and nothing else
- [ ] Failure alarm

## Verify

- [ ] Kill one source, confirm the run completes and the others land
- [ ] Confirm an authored field is never overwritten by a derived value
- [ ] Confirm a no-change run opens no PR
