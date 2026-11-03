# Phase 9 — Tasks

Rendering-layer only. Work top to bottom; each task is independently
verifiable. Nothing here touches content, payloads, or AWS.

## Investigate first

- [x] Read the `Project` struct in `resume-core` to confirm its fields. Has
      `slug`, `name`, `status`, `url?`, `repo?`, `summary`, `highlights`. Link
      target: `url` then `repo`, else non-link.
- [x] Read `profile_header`, `experience`, `projects`, `skills`, and the
      certifications block in `render.rs`. The rail reuses `skills()` and the
      certifications markup verbatim.
- [x] Confirm the `button-link` class and the fit modal-upgrade wiring. The
      compare button keeps `id="fit-trigger"` and `href="/fit"`; `.button-link`
      already carries exactly the bordered-cyan treatment the design wanted, so
      no new button CSS was needed.

## Theme

- [x] Remap the resume's `:root` tokens to the lab palette per the design
      table. Semantic names kept; comment records warm=structure,
      cool=interactive, names predate the reskin.
- [x] Dark-only: removed the `prefers-color-scheme: light` screen block (per
      the design's recommended option).
- [x] Print stylesheet re-asserted light — the `@media print` block already set
      its own light `:root` values explicitly, so it never relied on the
      removed light screen block. Single column, decoration off confirmed.
- [x] No hard-coded hex outside the `:root` and `@media print` token blocks.

## Layout

- [x] Added the `.resume-body` two-column grid (one column below `60rem`,
      `minmax(0,1.6fr) / minmax(0,1fr)` above, `align-items: start`).
- [x] Wrapped experience (left column) and the right rail in `.resume-body` in
      `page()`. Profile header full width above; fit dialog after.

## Right rail

- [x] Added `right_rail(resume)` rendering, in order: certifications, project
      links, the two action buttons, skills.
- [x] Certifications block in the rail (same markup).
- [x] `project_links()`: a `button-link` per project to `url` or `repo`; a
      non-link name+summary item when it has neither.
- [x] Two action buttons: "Compare a job description" (`id="fit-trigger"`,
      `href="/fit"`) and "Explore the GenAI lab" (`href="/gen-ai"`), in a
      `nav.rail-actions`.
- [x] Skills block in the rail, last.

## Removals

- [x] Deleted the "Hiring?" section and its paragraph from `page()`.
- [x] Deleted the provenance paragraph, form, and button from `page()`.
      `PageContext.provenance_enabled` kept (used by other pages).
- [x] No dangling reference to the removed copy remains in the resume page.

## Styling the rail and buttons

- [x] Action buttons and project links styled as block `button-link`s; the
      existing `.button-link` focus/hover covers the visible ring.
- [x] Rail rhythm tightened (`.rail-actions` spacing, first-heading top margin
      reset) so skills rises toward the fold; spacing scale only, no magic px.

## Tests

- [x] Rewrote the provenance test as `the_resume_page_never_offers_a_provenance_trigger`:
      no trigger regardless of the flag; fit link still present.
- [x] No old "Hiring?" or single-column order assertion remained to update
      (the removed block had no dedicated test beyond the provenance one).
- [x] Added `the_resume_links_to_the_gen_ai_lab`.
- [x] Added `the_right_rail_orders_certifications_then_projects_then_skills`
      (byte-position assertion on the headings).
- [x] `cargo test -p resume-core` green (130). `cargo clippy --all-targets
      --all-features -- -D warnings` clean. `cargo fmt --check` clean.
- [ ] No `insta` snapshot of the full rendered page exists to regenerate; the
      structural tests cover the change. If a page snapshot is added later, it
      captures the new layout.

## Browser verify (manual, recorded)

- [ ] Dark palette renders; links/buttons are cyan, headings magenta.
- [ ] Two columns at ≥60rem, one column below, with a sensible stacked reading
      order (experience, then rail).
- [ ] Right rail order correct; "Skills" at or above the fold on a 1366×768
      laptop (~600px usable height).
- [ ] "Compare a job description" opens the fit modal; works as a plain `/fit`
      link with JS off. "Explore the GenAI lab" navigates to `/gen-ai`.
- [ ] Keyboard: every button/link focusable with a visible ring.
- [ ] Contrast: body ≥4.5:1, large/UI ≥3:1, checked against computed values on
      the new surfaces, in the default theme.
- [ ] `prefers-reduced-motion`: no motion introduced, or it is opt-in by
      absence.
- [ ] Print preview is light, single column, no decoration.

## Out of scope

- Project pages / internal project routes (project links go to `url` or render
  as non-links until a later phase adds pages).
- Removing the provenance feature or route — only its resume-page trigger.
- A light screen theme (additive later if wanted).
- The PDF rendering pipeline beyond the print stylesheet staying light.
