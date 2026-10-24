# Phase 3 — Tasks

Status: not started

## Design system

- [ ] Token set: surfaces, text, accents, states, spacing scale, type scale
- [ ] Decide the palette ramps in `oklch()` with hex fallbacks
- [ ] Give the two accents distinct jobs: warm for structure, cool for
      interactive
- [ ] Fix the muted grey to clear 4.5:1 on the raised surface
- [ ] Light theme as a separate token set
- [ ] Verify contrast by computed value in both themes
- [ ] Pick the mono face, subset and self-host it, or fall back to system

## Markup and rendering

- [ ] Document shell with semantic landmarks and heading outline
- [ ] Profile header with hex-framed portrait
- [ ] Timeline: roles and career breaks interleaved by date, different markup
      per kind
- [ ] Skill groups, project cards, footer with commit SHA
- [ ] Bracket motif in templates and CSS
- [ ] Hexagon via `clip-path`, `aria-hidden` where decorative
- [ ] `insta` snapshot tests over rendered output

## Theme

- [ ] Cookie read in the handler, `data-theme` in the initial HTML
- [ ] `prefers-color-scheme` fallback expressed in CSS, not the handler
- [ ] `POST /theme` working without JavaScript
- [ ] Cache correctness: `Vary: Cookie` or cookie in the cache key

## Delivery

- [ ] Critical CSS inlined, one content-hashed stylesheet
- [ ] Portrait: responsive `srcset`, explicit dimensions, modern format with
      fallback, meaningful `alt`
- [ ] Performance budget check: first view under 100 KB

## PDF

- [ ] Separate rendering: light, single column, no portrait, no brackets
- [ ] Confirm it parses as text, not an image

## Infrastructure

- [ ] Domain and Route 53 hosted zone
- [ ] ACM certificate in `us-east-1`
- [ ] CloudFront distribution with behaviors in the documented order
- [ ] S3 static assets bucket with OAC
- [ ] Restrict the Function URL once CloudFront fronts it

## Content and SEO

- [ ] Metadata, Open Graph, `sitemap.xml`, `robots.txt`
- [ ] Accessibility pass: focus, keyboard, reduced motion, landmarks
