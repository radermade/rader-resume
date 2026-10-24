---
inclusion: always
---

# Pipeline and Publishing Standards

Git is the source of truth. Everything deployed is derived from a commit, and
anything that cannot be rebuilt from a commit is a bug.

## Source of truth

- Authored content lives in `content/` in this repo: the resume document,
  project write-ups, assets. The repo is canonical.
- S3, DynamoDB, CloudFront, and rendered artifacts are derived caches. Deleting
  them must be recoverable by re-running the pipeline against a commit.
- Derived data from external sources (GitHub activity, feeds, cert expiry) is
  not authored content. It is fetched, stamped with provenance and a fetch
  time, and merged at publish. It never becomes the only copy of anything, and
  it never silently overwrites authored fields.
- When derived data should become permanent, the sync opens a pull request
  against `content/`. Bots propose; humans merge.
- Secrets never enter the repo. No credentials, no tokens, no private contact
  details in content files.

## Three kinds of state

Keeping these separate is what makes git-as-truth coherent. Confusing them is
how a portfolio ends up with content that exists only in a database.

- Authored content lives in git. Canonical, reviewed, permanent.
- Derived artifacts live in S3, keyed by commit SHA. Rebuildable from a commit
  at any time, safe to delete.
- In-flight flow sessions live in DynamoDB with a TTL. Ephemeral runtime state,
  not content, and disposable by design: losing one costs a visitor a restarted
  interaction and nothing else.

Nothing in the third category is ever backed up, migrated, or treated as a
record. Nothing in the first category is ever written by a running Lambda.

## Branching and commits

- `main` is always deployable. No direct pushes; branch protection on, PR
  required, CI green required.
- Short-lived feature branches. Content changes are PRs like code changes.
- Conventional commits. The commit subject is what shows up in the publish
  history, so write it for that audience.
- Content document versions are tagged when they represent a real resume
  revision worth citing.

## CI

- GitHub Actions with OIDC into a deploy role. No long-lived access keys in the
  repo or in GitHub secrets.
- On pull request: `cargo fmt --check`, clippy with warnings denied, tests,
  content schema validation, link check, and a rendered preview artifact.
  Nothing is published.
- On merge to `main`: build, deploy infrastructure, then trigger the publish
  pipeline with the merge commit SHA.
- CI never mutates production state outside the pipeline. The pipeline is the
  only writer.

## Publish pipeline

The state machine, invoked with a commit SHA:

```
Resolve SHA → Ingest content → Normalize → Validate
            → Merge derived data → Render artifacts
            → Write artifacts (keyed by SHA) → Flip manifest
            → Invalidate cache → Notify
```

- Every run is identified by the commit SHA. Execution names derive from it so
  duplicate triggers collide rather than double-publish.
- Artifacts are immutable and written under a SHA-scoped prefix. Publishing is
  the atomic flip of a manifest pointer, not an in-place overwrite.
- Validation failures stop the run before anything is written, and record a
  diagnostic the admin view can read. A broken content document never reaches
  the live site.
- Validation rejects unresolved authoring placeholders. Any `[bracketed]`
  value in a content document fails the run and fails CI. A resume PDF that
  reaches a recruiter containing `[Current Title]` is a worse failure than any
  outage this pipeline is designed to prevent.
- The whole pipeline is idempotent. Re-running the same SHA produces the same
  result and is always safe.
- Asset filenames are content-hashed with long TTLs; HTML and API responses use
  short TTLs with ETags. Cache invalidation is targeted at the paths that
  changed, not a blanket wildcard.

## Rollback

- Rollback is re-publishing a previous SHA: flip the manifest back. It does not
  require a revert commit, a rebuild, or a redeploy.
- A revert commit follows afterwards so the repo matches reality.
- Keep enough SHA-scoped artifact generations to make this real. Lifecycle
  rules expire the old ones, not the current and previous.

## Stages

- Two stages: `dev` and `prod`, fully separate stacks. Same IaC, different
  parameters.
- A SHA is promoted, not rebuilt. What ran in dev is what publishes to prod.
- Prod publishing is automatic on merge to `main` until that becomes a problem.
  If a manual gate is added, it is a wait-for-approval state in the pipeline,
  not a human running commands.

## Provenance

- The live site exposes the commit SHA and publish timestamp it was built from,
  somewhere unobtrusive. It makes "is my change live?" a lookup instead of a
  guess.
- Pipeline logs carry the SHA on every entry so an execution can be traced back
  to the exact content that produced it.
