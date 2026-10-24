# Phase 3 — Public Site

Status: not started

Goal: something worth linking to.

## Functional

- R3.1 Server-rendered HTML for the full resume: profile, experience with
  career breaks interleaved, skills, projects, certifications.
- R3.2 Reachable on a custom domain over HTTPS.
- R3.3 Dark theme by default, light theme available, preference persisted.
- R3.4 A portrait, hex-framed, present on the site.
- R3.5 Downloadable PDF resume as a distinct rendering.
- R3.6 SEO metadata, Open Graph tags, `sitemap.xml`, `robots.txt`.
- R3.7 The live commit SHA visible somewhere unobtrusive.

## Constraints

- R3.8 No JavaScript required to read any content. The theme toggle degrades to
  a form post.
- R3.9 Theme preference in a cookie, read server-side, so the initial HTML is
  already correct. No flash of the wrong theme.
- R3.10 First view under 100 KB total excluding the PDF.
- R3.11 Hand-written CSS with semantic tokens. No utility framework.
- R3.12 At most one self-hosted, subset web font.
- R3.13 Contrast at 4.5:1 for body text and 3:1 for large text and UI in both
  themes, verified by computed value rather than by eye.
- R3.14 Visible focus indicators, full keyboard operability,
  `prefers-reduced-motion` honored.
- R3.15 The PDF is light regardless of site theme, single column, real embedded
  text, standard headings, no portrait, no decorative brackets.
- R3.16 CloudFront in front of everything, with specific path behaviors ordered
  ahead of the catch-all.
- R3.17 Assets content-hashed with long immutable TTLs; HTML short TTL with
  ETag.

## Exit criteria

Public site on the real domain, readable without JavaScript, passing contrast
checks in both themes, with a PDF that parses cleanly as text.

## Out of scope

Fit analysis, the publish pipeline, enrichment.
