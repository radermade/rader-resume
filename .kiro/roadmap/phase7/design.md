# Phase 7 — Design

Status: outline. Not worth elaborating until Phase 4 exists, since most of this
phase observes the pipeline.

## Transparency page

The idea worth keeping: expose the system's own operation as site content.
Recent publish executions with their steps, durations, and outcomes, plus the
commit SHA currently live.

It is cheap on top of what Phase 4 already produces and it does something no
written claim can, which is let a reviewer watch the architecture work. Pair it
with short architecture decision notes and the portfolio documents its own
engineering judgment.

Source: the publish pipeline writes an execution summary object alongside each
run, and the page reads those rather than calling `ListExecutions`. Same reason
as the Phase 6 read model, and it keeps the page cacheable.

## Admin, pending a decision

If git stays the only editing path, this phase has no auth work: the sync bot
needs a scoped token to open pull requests and nothing else needs a write
endpoint. That is the likely outcome and the cheaper one.

If an admin UI happens, it needs real authentication at the edge, re-checked in
the handler, and the write path has to go through the pipeline rather than
around it.

## Not yet designed

Dashboards, alarm routing, integration test harness, IAM review. All are
straightforward once there is something running to observe.
