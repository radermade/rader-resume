# Phase 7 — Admin, Hardening, Operations

Status: outline only. Elaborate when Phase 4 lands.

Goal: the system is operable and defensible.

## Functional

- R7.1 Observability: structured logs with correlation IDs, X-Ray on the state
  machines, dashboards worth opening.
- R7.2 Alarms on pipeline failure, function errors and throttles, and
  CloudFront 5xx, routed somewhere actually seen.
- R7.3 A pipeline transparency page: recent publish executions with steps,
  durations, outcomes, and the live commit SHA.
- R7.4 Architecture decision notes published as site content.
- R7.5 Integration tests against a deployed stage.

## Constraints

- R7.6 Least-privilege IAM per function, no wildcard resources.
- R7.7 Any write path requires real authentication, enforced at the edge and
  re-checked in the handler.
- R7.8 Log retention set explicitly everywhere.
- R7.9 Cost stays near free tier outside of LLM usage.

## Open

- R7.10 Whether an admin UI exists at all. Git-as-truth points at editing
  through commits and pull requests, which would reduce this phase to the sync
  bot's permissions and no auth work. This is roadmap open question 5 and it
  materially changes the phase's size.
- R7.11 Root credential cleanup: Identity Center or an IAM user with MFA, then
  delete the root access keys. Carried from Phase 1 setup.
