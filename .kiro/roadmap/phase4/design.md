# Phase 4 — Design

Status: shape settled, details open

## Workflow

```
Resolve SHA
  -> Ingest          read content at the SHA, write normalized copy to S3
  -> Validate        schema + resume-core validate() + link check
                     any issue -> RecordFailure
  -> MergeDerived    overlay enrichment data with provenance
  -> Render          Map over artifact kinds: html, json, pdf, og-image
  -> Write           under s3://artifacts/<sha>/
  -> FlipManifest    atomic pointer update
  -> Invalidate      targeted paths only
  -> Notify
Catch on every task -> RecordFailure
```

## Why a state machine here

Multiple steps with distinct failure modes, fan-out across artifact kinds,
retries that matter, and a run worth inspecting after the fact. That is four of
the five justifications, so the orchestration earns its place.

## Manifest flip

`s3://artifacts/current.json` holds `{ sha, publishedAt, artifacts }`. Readers
resolve content through it. Publishing writes a new manifest; rollback writes
an old one. Both are single small object puts, which is as close to atomic as
this needs.

Keep the current and several previous SHA generations. Lifecycle rules expire
older ones, never the current or previous, or rollback becomes a lie.

## Binaries

One per task state, per the Rust standards: `tasks/ingest`, `tasks/validate`,
`tasks/render`, `tasks/publish`. `RecordFailure` can be a direct SDK
integration rather than a Lambda if it only writes a diagnostic object.

`FlipManifest` and `Invalidate` are candidates for direct SDK integrations too.
Any task that would only forward a single call should not be a Lambda.

## Rendering versus the request path

Phase 3 renders on request from embedded content. Phase 4 renders artifacts
ahead of time. Both call the same `resume-core` functions, so there is one
source of markup and the snapshot tests cover both.

Open question: whether the site handler should serve the pre-rendered artifact
rather than rendering per request. Serving the artifact makes the pipeline
authoritative and the handler trivial, but reintroduces a runtime fetch that
embedding was chosen to avoid. Decide before building, not after.

## Open questions

- Trigger mechanism: CI calls `StartExecution` directly, or CI pushes a commit
  marker to S3 and EventBridge triggers. Direct is simpler; the marker gives a
  replay trail.
- Link checking against external URLs will be flaky. Decide whether a dead
  external link fails the publish or only warns.
- Whether `dev` publishes at all, or only `prod`.
