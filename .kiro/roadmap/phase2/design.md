# Phase 2 — Design

Status: partially implemented

## Content embedded at compile time

`resume-core` pulls `content/resume.json` in with `include_str!`.

Git is the source of truth and the binary is built from a commit, so embedding
ties served content to the SHA that produced it. No runtime fetch, no
cold-start I/O, no drift between what the repo says and what the Lambda serves.
S3 stays for derived artifacts only.

The tradeoff: content changes require a rebuild. That is correct here, because
publishing is already a build-and-deploy triggered by a commit.

## Module split

```
resume-core/src/lib.rs        model types, CONTENT, load()
resume-core/src/dates.rs      ImpreciseDate, RoleEnd, serde and ordering
resume-core/src/validate.rs   ValidationIssue, validate()
```

## Dates

`ImpreciseDate` is `Year(u16) | YearMonth(u16, u8)` with a string wire form.
Hand-written `Serialize`/`Deserialize` delegate to `Display`/`FromStr`, because
no derive produces a single-string representation: an untagged enum over
`Year(u16)` expects a JSON number and the document holds `"2021"`.

`Ord` is manual. The derived implementation compares variant order first, which
would sort every bare year ahead of every year-month regardless of value.
Month validation to 1-12 makes the sort key `(year, month)` injective over
valid values, which keeps `Ord` consistent with `Eq`.

## Validation seam

`validate()` returns `Vec<ValidationIssue>`, each with a path and a detail.
Two distinct failures: empty, and unresolved placeholder.

Placeholder detection keys on square brackets. That works precisely because
the bracket motif was pushed into templates and CSS, so a bracket in the record
can only be an unfilled blank. This is a dependency between the design standard
and the validator; if brackets ever enter content legitimately, the detector
needs a different marker.

The unit suite asserts the detector catches a known placeholder rather than
asserting the document is clean. Phase 4's pipeline treats a non-empty result
as fatal.

## Read path

`LazyLock<Resume>` parsed once at cold start. A parse failure is a build-time
content error, so panicking is correct: the function should not serve a
half-understood document.

## Remaining decisions

- JSON Schema export: `schemars` derive versus a hand-written schema. Derive
  couples the schema to the types, which is the point, but adds a dependency to
  a crate that is deliberately thin.
- ETag source: hash of the serialized document, or the build's commit SHA. The
  SHA is cheaper and already needs to be surfaced for the footer.
