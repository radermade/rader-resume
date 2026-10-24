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

## Rendering moves here

Resolved: the pipeline becomes the only renderer, and reads become static.

Phase 3 renders on request from content embedded in the binary. That is a
stepping stone. Here, `tasks/render` produces the HTML and JSON artifacts, and
CloudFront serves them from S3 behind the manifest. Reads cost zero Lambda
invocations and the handler keeps only the fit analysis.

Both callers use the same `resume-core` functions, which is why this is a
change in caller rather than a rewrite, and why the snapshot tests keep working
across the move.

Consequences worth planning for:

- `/api/resume`, `/api/projects`, and `/api/schema` become artifacts, not
  handlers. Their ETags come from the artifact, so the fingerprint logic moves
  into the render task.
- CloudFront behaviors invert: S3 becomes the default origin and the function
  gets specific paths, rather than the other way round.
- The embedded `include_str!` content stays useful for the fit analysis, which
  needs the corpus in process to match against.
- Nothing visitor-specific may render into those artifacts. That constraint is
  recorded in the design and frontend steering, and theming is where it bites.

## Open questions

- Trigger mechanism: CI calls `StartExecution` directly, or CI pushes a commit
  marker to S3 and EventBridge triggers. Direct is simpler; the marker gives a
  replay trail.
- Link checking against external URLs will be flaky. Decide whether a dead
  external link fails the publish or only warns.
- Whether `dev` publishes at all, or only `prod`.
