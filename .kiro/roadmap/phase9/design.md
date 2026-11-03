# Phase 9 — Design

How the reskin and the split screen are built, and the decisions that are
easy to get wrong.

## Where the change lives

Two files do almost all of it:

- `crates/resume-core/src/styles.css` — the token remap and the new grid.
- `crates/resume-core/src/render.rs` — the `page()` body: the two-column
  wrapper, the right rail, the removed sections, the project links, the two
  buttons.

No content document changes. No payload type changes. `resume-core` stays free
of AWS and lambda types, as always. The handler is untouched: it already calls
`page()`; the page's innards change beneath it.

## Theme port

### The decision: remap values, keep names

The two apps keep separate token sets on purpose — the steering is explicit
that neither is the other's fallback, so a change to one cannot silently drift
the other. Porting the look therefore means copying the lab's *values* into the
resume's *semantic names*, not importing the lab's `--lab-*` properties.

The resume's component rules already reference `--surface`, `--text`,
`--accent-warm`, `--accent-cool`, etc. If those names keep their meaning and
only change value, the component CSS barely moves. That is the whole reason the
design standards insist on semantic names over literal ones, and this phase is
the payoff.

Mapping from the lab palette to the resume's semantic slots:

| Resume token | New value | Lab source | Job |
| --- | --- | --- | --- |
| `--surface-sunken` | `#000000` | `--lab-void` | the page behind everything |
| `--surface` | `#0d0f12` | `--lab-surface` | the card/section surface |
| `--surface-raised` | `#16191f` | `--lab-raised` | raised elements |
| `--border` | `#262b33` | `--lab-border` | hairlines |
| `--text` | `#d7dae0` | `--lab-text` | body |
| `--text-muted` | `#9aa0ab` | `--lab-text-muted` | secondary |
| `--accent-warm` | `#ff4fd8` | `--lab-prompt` | structure: headings, bracket motif |
| `--accent-cool` | `#4fe3ff` | `--lab-code` | interactive: links, buttons, focus |

The accent *names* are now slightly misleading — `--accent-warm` holds a
magenta, `--accent-cool` a cyan — but renaming them is a churn across every
component rule for no behavioral gain. The design standards care that the two
accents have fixed, distinct jobs, and they still do: structure vs.
interactive, the same split the lab enforces. A one-line comment at the token
block records that the "warm/cool" names predate the reskin and now mean
"structure/interactive." Renaming is a separate, optional cleanup.

### Light theme

The lab is dark-only. The resume currently ships a `prefers-color-scheme:
light` token set. Two honest options:

1. **Keep a light set, re-derived.** The lab has no light palette to copy, so a
   light variant would be newly designed, not ported. This keeps parity with
   the current resume behavior and respects a visitor who forces light. Cost: a
   second palette to design and contrast-check, for a scheme whose whole
   identity is the dark terminal look.
2. **Dark-only, drop the light block.** Simpler, and truer to the identity
   being adopted — the lab made this call. A light-preferring visitor gets the
   dark scheme, which is a defensible choice for a site whose point is the dark
   aesthetic, and it still meets contrast because the dark scheme does.

Recommendation: option 2 for the screen theme, dark-only, and lean on the
**print stylesheet** (R9.17) to cover the one place a light rendering genuinely
matters — paper. This matches the lab, removes a palette's worth of
contrast-checking, and keeps the static-first constraint trivially (one scheme,
no cache-key variants). If a light screen theme is wanted later it is additive.

Either way, R9.4 holds: no per-request rendering, no `localStorage` toggle (the
flash-of-wrong-theme failure the design standards name).

### Print

The print stylesheet stays light. If option 2 drops the light screen tokens,
the print block must set its own light values explicitly rather than relying on
a `prefers-color-scheme` block that no longer exists. A resume printed in the
dark palette wastes toner and reads as a mistake. The print rules are scoped to
`@media print` and override the dark tokens with light values plus the
single-column, decoration-off treatment the design standards already specify.

## Split-screen layout

### The grid

The body becomes a two-column grid at a breakpoint. Following the lab's landing
split, which collapses to one column below `60rem`:

```css
.resume-body {
  display: grid;
  grid-template-columns: 1fr;
  gap: var(--space-5);
}

@media (min-width: 60rem) {
  .resume-body {
    grid-template-columns: minmax(0, 1.6fr) minmax(0, 1fr);
    align-items: start;
  }
}
```

Experience is the wide left column; the right rail is the narrower right one.
`align-items: start` so the rail does not stretch to the experience column's
height. `minmax(0, …)` so a long preformatted line in experience cannot blow
the track out.

The profile header stays full width above the grid. The fit dialog stays where
it is (a modal is not in the flow). So `page()` becomes:

```
profile_header
.resume-body {
  <left>  experience
  <right> right_rail
}
fit_dialog
```

### Source order and accessibility

The grid is laid out in source order: experience first, right rail second. On a
narrow screen that is the reading order — experience, then certifications,
projects, buttons, skills — which is a sensible resume order. R9.14 is
satisfied by keeping source order == reading order and not reordering with
`order` or `grid-row` tricks that desync the visual and DOM order.

Each column is a landmark-bearing region. Experience and the rail each get a
heading; the rail's children keep their `h2`/`h3` outline. No `role` gymnastics
needed — a two-column grid of two `<section>`-ish regions reads correctly.

### The right rail, top to bottom

A new `right_rail(resume, ctx)` function renders, in order:

1. **Certifications** — the existing certifications `section`, moved out of the
   main flow into the rail. Same markup.
2. **Project links** — a new presentation: a list of links, one per project,
   into the project's destination (see below). Not the inline `projects()`
   write-up.
3. **Action buttons** — "Compare a job description" (the fit trigger) and
   "Explore the GenAI lab" (→ `/gen-ai`), as a small button cluster.
4. **Skills** — the existing `skills()` section, moved into the rail last.

The order is deliberate and is the fold argument: certifications and projects
are short, so skills rises close to the fold beneath them. If in-browser
checking (R9.8) shows skills sitting too low, the lever is tightening the
spacing above it or moving the button cluster, not reordering — the order is a
stated requirement.

### The fold (R9.8)

Not unit-testable. The design assumption: a 1366×768 laptop, browser chrome
taken off, ~600px of usable height. The check is manual in the browser at that
size, confirming the "Skills" heading is visible without scrolling, or within a
small tolerance. Recorded in `tasks.md` as a browser-verify step, the same way
Phase 2's lab layout was checked. No pixel value is hard-coded in CSS to force
it; the order and spacing produce it, and the spacing scale is the only knob.

## Project links

R9.12 wants projects as links, not prose. The destination question: the resume
content has `project.url` (external) and a slug. Options for the link target,
in preference order:

1. If a project has a `url`, link to it.
2. Otherwise, link to an on-site project anchor or page if one exists.

Since this phase does not add routes, the safe scope is: link each project to
its `url` when present, and for a project with no `url`, render it as a
non-link list item (its name and one-line summary) rather than a dead link. A
later phase can add project pages and turn these into internal links. The
implementation reads the project struct to confirm which fields exist before
committing to this; `tasks.md` has a step to verify the field set.

The list is styled as buttons/links consistent with the two action buttons, so
"buttons to projects" reads as intended without implying each is a form.

## Removals

From `page()`:

- The entire `section { h2 { "Hiring?" } … }` block.
- The `provenance_enabled` branch inside it (the paragraph, the `form`, the
  button).

Kept:

- `ctx.provenance_enabled` on `PageContext` — other pages/plumbing use it, and
  the field is harmless. Only its *use in the resume page body* is removed. The
  test `provenance_trigger_is_absent_when_unavailable` is rewritten: the
  resume page now never shows a provenance trigger regardless of the flag, so
  the test asserts absence unconditionally (and still asserts the fit link is
  present).
- The fit feature end to end. `fit_dialog()`, the `/fit` route, the modal
  upgrade. Only the trigger's label and location change.

## Buttons

Both action buttons share a `button-link` treatment (the class the current fit
trigger already uses). The `/gen-ai` one is a plain `<a href="/gen-ai">`. The
compare one keeps `id="fit-trigger"` and `href="/fit"` so the existing
modal-upgrade JavaScript still finds it — that wiring is untouched, which is
why the fit behavior needs no change.

Button styling uses `--accent-cool` (interactive), a bordered, transparent-fill
treatment like the lab's `.lab-nav button`, so the two apps' buttons rhyme.

## Tests

- Update `provenance_trigger_is_absent_when_unavailable` per above.
- Update any test asserting the old section order or the "Hiring?" string.
  Expect `the_stack_page_is_reachable_from_the_resume` and the fit-field tests
  to be unaffected (the fit link and `/stack` link survive).
- Add a test that the resume page contains a link to `/gen-ai` — an unlinked
  lab may as well not exist, the same rationale as the existing `/stack` test.
- Add a test that the right-rail sections appear in the required order
  (certifications before projects before skills in source order), by asserting
  the byte positions of their headings.
- Regenerate the `insta` snapshot of the rendered page and review the diff.
- Contrast is verified manually against computed values (R9.3); note it in the
  browser-verify step rather than asserting it in a unit test.

## Risks

- **Accent name drift.** `--accent-warm` now holds magenta. Mitigated by the
  comment and by not renaming (renaming is the larger churn). Flagged so a
  future reader is not misled.
- **Contrast on the darker surface.** The resume's muted text value is replaced
  by the lab's, which was chosen for `#0d0f12`; confirm it still clears 4.5:1
  wherever muted text lands on the resume (it may sit on a different surface
  than it does in the lab).
- **Fold is viewport-dependent.** R9.8 is a goal, not a guarantee across every
  device. The design states the reference viewport so "above the fold" has a
  definition rather than being an argument.
- **Print regression.** Dropping the light screen tokens means the print block
  must assert light values itself. Easy to forget; it is an explicit task.
