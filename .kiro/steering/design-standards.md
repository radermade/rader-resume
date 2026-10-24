---
inclusion: always
---

# Design Standards

This is a frontend engineer's portfolio. The visual craft is part of the claim,
so a technically immaculate ugly site argues against its own author. Design
decisions get the same rigor as the architecture ones.

## Direction

Dark-first developer aesthetic. Restrained, high contrast, typographic rather
than decorative.

Two motifs are worth evolving from the previous site: the angle bracket framing
on headings, as in `< Experience >`, and the hexagon. Neither is a requirement.
They are the parts of the old site that had character, and the goal is to do
them better rather than reproduce them.

Both belong in templates and CSS, never in content data, which is why the
brackets were stripped out of `resume.json`. The personality lives in the
rendering layer where it can change without touching the record.

## Theming

- Dark is the default, not a toggle away from light.
- Respect `prefers-color-scheme` for first-time visitors with no preference
  set.
- An explicit toggle overrides it, and the choice persists in a cookie rather
  than `localStorage`. This matters because the site is server-rendered: the
  handler reads the cookie and emits the correct theme in the initial HTML.
  A `localStorage` toggle would produce a flash of the wrong theme on every
  page load, which is the most common way SSR sites give themselves away.
- The toggle degrades without JavaScript. A form post that sets the cookie and
  re-renders is acceptable and costs one round trip.
- Never theme by swapping stylesheets. One stylesheet, custom properties
  redefined under a `[data-theme]` attribute on the root element.

## Tokens

- All color, spacing, type scale, and radius values are CSS custom properties
  defined once at the root. No hard-coded hex values in component rules.
- Semantic names, not literal ones: `--surface-raised`, not `--gray-800`. The
  literal palette maps into semantic tokens in one place.
- Spacing comes from a scale. Arbitrary pixel values in layout rules are a
  smell.

## Palette

The previous site's scheme is the reference point, not the specification. What
was good about it: neutral charcoals with a warm terracotta and a cool sky blue
as a complementary accent pair, both of which measure around 7.6:1 against the
darkest surface. Accents that saturated usually fail contrast, so that
relationship is worth preserving even if the exact values change.

Starting values, expected to move:

```css
--surface-sunken: #141414;  --text:        #cfd0d0;
--surface:        #242424;  --text-muted:  #8a8a8a;
--surface-raised: #2f3948;  --accent-warm: #df966c;
                            --accent-cool: #6fafd3;
```

Where to improve on it:

- Give the two accents distinct jobs. The old site used both for headings and
  highlights interchangeably. Warm for structure, cool for anything
  interactive, and never the reverse. Consistency is what makes a two-accent
  scheme read as designed rather than decorated.
- Define the ramps in `oklch()` rather than hand-picked hex. Perceptually
  uniform steps mean a lightness change does not drag saturation with it, which
  is the usual reason hand-tuned dark palettes look muddy in the midtones.
- Unify the surface hues. Two neutral greys plus one blue-grey reads as
  accumulated rather than chosen. Pick a slight consistent hue shift across all
  surfaces; fully neutral dark greys look flat.
- Build a real elevation scale instead of three ad-hoc surfaces.
- Add the state colors the old palette never had: focus, error, success, and
  visited. The fit analysis form needs all of them and guessing later produces
  colors that fight the scheme.
- Fix the muted grey. `#898989` cleared AA on the darkest background but landed
  near 4.4:1 on the `#242424` surface, missing body text contrast by one hex
  step. This is the failure mode the accessibility section warns about: dark
  themes break on secondary text, not on the obvious elements.
- Light theme is a separate token set, not an inversion. Inverting a dark
  palette produces washed-out accents every time.

## CSS

- Hand-written CSS with custom properties, grid, and flexbox. No utility
  framework.
- Rationale: `maud` templates with long utility class strings read badly, and
  on a frontend portfolio, demonstrable CSS fluency says more than a config
  file does. This is a deliberate tradeoff against Tailwind's market
  recognition.
- One stylesheet, content-hashed filename, long immutable cache TTL.
- Critical CSS inlined in the server-rendered document. The handler controls
  the whole response, so there is no excuse for a render-blocking stylesheet
  request.
- Logical properties (`margin-inline`, `padding-block`) over directional ones.

## Typography

- System font stack for body text. No render-blocking web font request for a
  page whose job is to be read immediately.
- At most one web font, self-hosted, subset to the characters actually used,
  preloaded, `font-display: swap`. If it costs more than a few KB after
  subsetting, fall back to the system stack.
- Inconsolata set the tone on the old site and is a reasonable default, but it
  is not a commitment. A variable mono with weight and width axes gives more
  range from the same payload, and there are more current options worth
  comparing before settling.
- Monospace for structure: labels, dates, section headings, the bracket motif.
  Prose gets a readable text face. The old site ran mono for nearly everything,
  which looked right and read worse, and the resume content is scanned under
  time pressure.
- Measure capped around 70 characters for prose. Resume content is scanned, not
  read linearly, so prioritize clear hierarchy over density.

## Hexagon motif

The hexagons are worth keeping if they earn their place. The upgrade is giving
the shape a job instead of scattering it: the portrait frame, skill cluster
cells, or timeline nodes. A motif that encodes something beats fourteen
floating ornaments, and the ornaments are what forced the hand-positioning in
the first place.

The old implementation stacked three rotated rectangles per shape, positioned
every instance with a hardcoded numbered ID rule, and reused `id="color1"`
across seven elements, which is invalid HTML. One element and one rule replaces
all of it:

```css
.hex {
  aspect-ratio: 1.1547;  /* 2/√3, flat-top regular hexagon */
  clip-path: polygon(25% 0, 75% 0, 100% 50%, 75% 100%, 25% 100%, 0 50%);
}
```

- No pseudo-element construction, no per-instance ID rules, no absolute pixel
  positioning. Lay clusters out with grid and let the offset come from the
  grid, not from hand-tuned coordinates.
- Decorative hexagons are `aria-hidden` and carry no content.
- Honor `prefers-reduced-motion` if any of them move. They should probably not
  move.

## Photography

A portrait belongs on the site. It humanizes a page that is otherwise a wall of
claims, and a personal portfolio is the right venue for it.

- Hex-clipping the portrait is the obvious way to resolve the motif and the
  photo into one decision rather than two competing ones. Worth trying first,
  not worth forcing if it looks contrived.
- A fresh photo is probably worth it. The old one is at least eight years old
  and the current role is the one being sold.
- Content-hashed filename, explicit width and height to prevent layout shift,
  `srcset` for density, modern format with a fallback, and meaningful `alt`
  text describing the person rather than saying "photo".
- No portrait in the resume PDF. US hiring convention omits it, image handling
  in resume parsers is unreliable, and including a photo in screening material
  introduces bias risk you gain nothing from. Site yes, PDF no. This is one of
  the concrete reasons the PDF is a separate rendering rather than a print
  button.

## Accessibility

- Contrast: 4.5:1 for body text, 3:1 for large text and UI boundaries, in both
  themes. Dark themes fail this quietly, especially on muted secondary text, so
  check the actual computed values rather than trusting that it looks fine.
- Visible focus indicators on every interactive element. Never remove an
  outline without replacing it with something at least as clear.
- Full keyboard operability, including the theme toggle and the fit analysis
  form.
- Honor `prefers-reduced-motion`. Any transition or animation is opt-in by
  absence of that preference.
- Semantic landmarks and a sensible heading outline. The resume is a document;
  mark it up like one.
- Do not claim WCAG compliance. Validation requires manual testing with
  assistive technology and expert review.

## Print and PDF

- A separate print stylesheet, light regardless of the site theme. Nobody wants
  a dark-mode resume through a printer.
- The PDF resume is a distinct rendering, not the page with a print button:
  single column, real embedded text, standard section headings, no decoration
  carrying meaning, nothing load-bearing inside a table or graphic. It gets
  parsed by software, so the bracket motif comes off here.

## Performance budget

- First view under 100 KB total, including HTML, CSS, and any image, excluding
  the PDF download.
- No JavaScript required to read any content. htmx enhances the fit analysis
  flow and the theme toggle; neither depends on it.
- Images content-hashed, sized, lazy below the fold, modern formats with
  fallbacks.
