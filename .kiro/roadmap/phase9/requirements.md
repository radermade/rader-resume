# Phase 9 — Resume reskin and split-screen layout

The resume adopts the GenAI lab's visual identity and a two-column layout. The
lab's terminal-dark scheme reads as more considered than the resume's warmer
charcoal, and a frontend engineer's portfolio is judged partly on its surface,
so the surface is worth the work.

This is a rendering-layer change. No content document, no payload type, and no
`resume-core` data structure changes. The resume record is untouched; only the
templates and CSS that render it move.

## Goals

1. The resume uses the lab's color scheme — the same surfaces, the same two
   accents with the same jobs — so the two apps read as one product rather than
   two sites that happen to share a domain.
2. The resume body is a split screen on wide viewports: experience on the left,
   a right rail on the right. One column on narrow viewports.
3. The right rail, top to bottom: certifications, project links, the two action
   buttons, then skills. Skills clears the fold or very nearly does on a
   laptop viewport.
4. The two action buttons are "Compare a job description" (the fit feature,
   renamed from the "Hiring?" framing) and a link to `/gen-ai`.
5. The "Hiring?" section and all of its framing are removed. The provenance
   check button and its copy are removed from the resume page.
6. Projects are presented as links/buttons into each project, not inline
   write-ups on the resume page.

## Non-goals

- No change to the content model, validation, or any Step Functions payload.
- No change to the fit analysis logic or the `/fit` endpoint. Only the button
  that opens it and its wording change.
- No removal of the provenance *feature* or its workflow — only its trigger on
  the resume page. The `/provenance` route and `provenance_enabled` plumbing
  stay; the resume simply stops surfacing a button for it.
- No change to the lab's own theme. The port copies values into the resume's
  token set; it does not make either app depend on the other's properties.
- The PDF/print rendering stays light and is a separate concern, unaffected by
  the dark reskin (see the print requirement below).

## Functional requirements

### Theme

- R9.1 The resume's semantic CSS custom properties keep their names
  (`--surface`, `--surface-raised`, `--text`, `--text-muted`, `--accent-warm`,
  `--accent-cool`, `--border`, the spacing and radius scale) and take new
  values matching the lab's palette. Component rules reference tokens, so the
  value remap is the change and the rules are largely untouched. No hard-coded
  hex outside the `:root` token block.
- R9.2 The two accents keep distinct jobs, consistent with the lab: one for
  structure (headings, the bracket motif), one for anything interactive
  (links, buttons, focus). Never the reverse.
- R9.3 Contrast holds in the new scheme: 4.5:1 for body text, 3:1 for large
  text and UI boundaries, checked against the actual computed values on the new
  surfaces, not by eye. The lab's muted text was chosen to clear this; the
  resume's must too on whatever surface it sits on.
- R9.4 Theming requires no per-request rendering. Dark is the default; the
  light-theme story is decided in design (either a darkened light token set as
  today, or dark-only with `prefers-color-scheme` left unhandled). Whichever is
  chosen, it must not force a per-visitor render, because the static-first
  target in Phase 4 forbids it.
- R9.5 Focus indicators remain visible on every interactive element in the new
  scheme, using the interactive accent.

### Layout

- R9.6 On viewports at or above a named breakpoint, the resume body is two
  columns: experience left, right rail right. Below it, one column with the
  right rail stacked after experience (or before — decided in design so the
  single-column reading order is sensible).
- R9.7 The right rail renders, top to bottom: certifications, then project
  links, then the two action buttons, then skills.
- R9.8 On a representative laptop viewport (design names the exact height
  assumption), the Skills heading is at or above the fold. This is a layout
  goal verified in the browser, not a unit-testable assertion; the design
  states how it is checked.
- R9.9 The layout is a progressive enhancement of a sensible single-column
  document. With no CSS (or a very narrow screen) the page is still a coherent,
  ordered resume. The split is grid, not absolute positioning.

### Content surfaces

- R9.10 "Compare a job description" is the fit trigger. It is a real link to
  `/fit`, upgraded to the existing modal where that works, exactly as the
  current `fit-trigger` is — only the label and its placement in the right rail
  change.
- R9.11 A button links to `/gen-ai`. It is a plain navigation link styled as a
  button, no JavaScript dependency.
- R9.12 Projects appear as a list of links, one per project, into whatever the
  project's destination is (its `url`, or a future project page — design names
  the target). No inline project prose on the resume page.
- R9.13 The "Hiring?" `h2` and its paragraph are gone. The provenance
  paragraph, form, and button are gone from the resume page. No dead reference
  to either remains in the page template.

### Accessibility and semantics

- R9.14 The two columns are real landmarks or a sensible heading outline, not
  two visual columns that read as one jumbled source order to a screen reader.
  Source order is the single-column reading order.
- R9.15 Every button and link is keyboard operable with a visible focus ring.
  The `/gen-ai` and compare buttons included.
- R9.16 `prefers-reduced-motion` is honored for any transition the reskin
  introduces. The reskin should introduce none that move; if it does, they are
  opt-in by absence of the preference.

### Print and PDF

- R9.17 The print stylesheet stays light regardless of the screen theme. A
  dark-mode resume through a printer is the failure the design standards call
  out. The reskin is screen-only; print is unaffected or explicitly re-asserted
  light.

## Acceptance

- The resume renders dark in the lab's palette, two columns on wide screens,
  one on narrow.
- The right rail order is certifications, project links, the two buttons,
  skills, with skills at or near the fold on a laptop.
- "Compare a job description" opens the fit modal; the `/gen-ai` button
  navigates. No "Hiring?" framing, no provenance button on the page.
- Contrast and focus verified in both the default and light paths.
- All existing `resume-core` tests pass, with the ones asserting the old page
  structure updated to the new structure rather than deleted.
- The snapshot of the rendered page is regenerated and reviewed.
