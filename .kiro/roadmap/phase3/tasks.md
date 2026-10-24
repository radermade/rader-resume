# Phase 3 — Tasks

Status: not started

## Design system

- [x] Token set: surfaces, text, accents, spacing scale, measure, fonts
- [x] Accents given distinct jobs: warm for structure, cool for interactive
- [x] Muted grey lifted from `#898989` to `#9a9a9a` to clear 4.5:1 on
      `--surface`
- [x] Light theme as a separate token set with darkened accents, not an
      inversion
- [x] Contrast verified by computed relative luminance, both themes:

      | Pair | Ratio |
      | --- | --- |
      | dark `--text-muted` on `--surface` | 5.5:1 |
      | dark `--accent-warm` on `--surface-sunken` | 7.6:1 |
      | dark `--accent-cool` on `--surface-sunken` | 7.7:1 |
      | light `--accent-warm` on `--surface` | 5.6:1 |
      | light `--accent-cool` on `--surface` | 7.2:1 |
      | light `--text-muted` on `--surface` | 7.1:1 |

      All clear AA for body text. Still worth a browser check against the real
      rendered page, since computed ratios do not catch antialiasing or
      overlapping surfaces.
- [ ] Decide the ramps in `oklch()`. Left as hex because the existing values
      are contrast-verified and converting them by guess risks muddy midtones
- [ ] Pick the mono face. Currently the system stack, which is fine and free

## Markup and rendering

- [x] Rendering moved into `resume-core::render`, taking typed data and a
      `PageContext`, with no AWS, HTTP, or environment types in signatures
- [x] Document shell with landmarks and a single `h1`
- [x] Timeline with roles and breaks rendered differently
- [x] Skill groups, project cards, footer with commit SHA and schema version
- [x] Bracket motif as CSS generated content with empty alt text, so assistive
      technology skips it
- [x] Hexagon via `clip-path`, one element and one rule, `aria-hidden`
- [x] Monogram placeholder until a portrait exists
- [ ] `insta` snapshot tests. Currently targeted assertions; snapshots need the
      dependency added

## Theme

- [x] CSS-only: dark default with a `prefers-color-scheme: light` override.
      No handler involvement, which is what keeps the static path open
- [ ] Decide whether a persistent manual toggle is worth the CloudFront
      cache-key variant. Default is no

## Delivery

- [x] Styles inlined into the document. One request, no render-blocking fetch
- [ ] Extract to a content-hashed asset if the stylesheet outgrows inlining
- [ ] Portrait: responsive `srcset`, explicit dimensions, modern format with
      fallback, meaningful `alt`
- [ ] Performance budget check: first view under 100 KB

## PDF

- [ ] Separate rendering: light, single column, no portrait, no brackets
- [ ] Confirm it parses as text, not an image

## Infrastructure

Ordered, because each step blocks the next.

- [x] `rader-resume.com` chosen. Registration stays at Squarespace, DNS
      delegates to Route 53, because an apex cannot be a CNAME
- [x] `DnsStack` in the bootstrap app with a retain policy
- [x] `SITE_URL` wired into the CI build, so canonical, `og:url`, and the
      sitemap start working on the next deploy
- [ ] `npm run deploy:dns`, then set the four nameservers from the stack output
      as custom nameservers at Squarespace. Propagation is hours
- [ ] Confirm delegation: `dig NS rader-resume.com` returns the Route 53
      nameservers
- [ ] ACM certificate in `us-east-1` via a cross-region stack. DNS validation
      fails until delegation has landed
- [ ] CloudFront distribution, behaviors in the documented order
- [ ] S3 static assets bucket with OAC
- [ ] Test against the distribution domain before touching the apex record
- [ ] Alias records for the apex and `www`
- [ ] Restrict the Function URL once CloudFront fronts it

## Content and SEO

- [x] Description, Open Graph, and Twitter card tags
- [x] `robots.txt` and `sitemap.xml`, both driven by an optional `SITE_URL`.
      Canonical, `og:url`, and the sitemap are omitted entirely without an
      origin, because a wrong canonical hurts more than a missing one and a
      relative sitemap entry is invalid
- [x] `prefers-reduced-motion` honored, `:focus-visible` outlines
- [ ] `og:image`. Needs the image artifact from Phase 4's render fan-out
- [ ] Accessibility pass against a real browser: keyboard traversal, screen
      reader read-through, zoom to 200%
