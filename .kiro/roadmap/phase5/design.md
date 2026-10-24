# Phase 5 — Design

Status: outline. Firms up once sources are chosen.

## Workflow

```
EventBridge Scheduler
  -> ListSources           from config in the repo
  -> Map over sources      ToleratedFailurePercentage set deliberately
       -> FetchSource      one source per invocation
       -> Normalize        into a derived-data type with provenance
  -> MergeAndDiff          compare against what is already in content/
  -> Choice: meaningful change?
       no  -> Done
       yes -> OpenPullRequest   bot proposes, human merges
```

Publishing is not part of this workflow. A merged PR triggers Phase 4, which is
the only writer. That keeps one publish path rather than two.

## Provenance

Every derived record carries the source, the fetch timestamp, and the fetching
prompt or query where applicable. A field whose origin cannot be explained does
not belong in a resume.

## Bot pull requests over direct writes

Git-as-truth means a running Lambda never writes authored content. The sync
proposing a PR preserves that: the diff is reviewable, the merge is a human
decision, and the audit trail is the commit history rather than a log.

The cost is latency and friction. Accepted, because the alternative is a resume
that edits itself.

## PDF branch

Separate from enrichment. It belongs with Phase 4's render fan-out rather than
here; listed in this phase only because the roadmap originally grouped them.
Resolve by moving it to Phase 4's artifact `Map` and deleting it from here.

## Open

- Source list and schedule.
- Whether to run `dev` on a schedule at all, or only `prod`.
- Rate limits and caching per source. GitHub's API in particular.
