# Phase 3 — Design

Status: not started

Design standards live in `.kiro/steering/design-standards.md` and
`.kiro/steering/frontend-and-routing.md`. This file covers what is specific to
building the site.

## Routes

```
GET  /                 full resume document
GET  /projects/{slug}  project detail
GET  /resume.pdf       redirect or proxy to the S3 artifact
POST /theme            set the theme cookie, redirect back
GET  /api/resume       JSON
GET  /api/projects     JSON
GET  /healthz
```

## Theme without a flash

The handler reads a `theme` cookie and emits `data-theme` on `<html>` in the
initial response. No client-side correction, so no flash.

Resolution order: cookie, then `prefers-color-scheme` via a media query in CSS
that only applies when the cookie is absent, then dark. Expressing the fallback
in CSS rather than in the handler keeps the server from needing to guess at a
client preference it cannot see.

`POST /theme` sets the cookie and redirects to the referrer. That is the
no-JavaScript path and it costs one round trip. htmx can swap it in place
later; the form works either way.

Theme responses must not be cached by CloudFront, and any page whose markup
varies by cookie needs either `Vary: Cookie` or a cache policy including the
cookie in the key. Getting this wrong serves one visitor's theme to another.

## Rendering

Markup functions live in `resume-core`, taking typed data and returning
`Markup`. Handlers wire requests to them and contain no markup.

Component seams: document shell, profile header, timeline (roles and career
breaks interleaved by date), skill groups, project cards, footer with the
commit SHA.

Snapshot tests with `insta` over rendered output.

## Timeline interleaving

Roles and career breaks merge into one date-ordered sequence, rendered with
different markup per kind. A break is not a job, so it gets no employer line
and no title. This is the structural reason `CareerBreak` is a separate type.

## CSS delivery

One stylesheet, content-hashed, long immutable TTL. Critical CSS inlined into
the SSR document since the handler controls the whole response.

Tokens defined once at `:root` with a `[data-theme="light"]` override block.
Light is a separate token set, not an inversion.

## PDF

A separate rendering path, not the page with print styles. Single column, real
text, standard section headings, no portrait, no bracket motif, light palette.

Generation belongs in the publish pipeline rather than the request path, since
it is slow and the output is an artifact keyed by commit SHA. Phase 3 can ship
with the PDF link pointing at an artifact the pipeline produces in Phase 4; if
Phase 3 lands first, generate it locally and commit it as a stopgap with a note.

## CloudFront

Behaviors, ordered:

1. `/api/*` to the function, short TTL, ETag driven
2. `/flow/*` and `/fit/*` to the function, no-store (reserved for Phase 6)
3. `/theme` to the function, no-store
4. `/static/*` to S3 with OAC, long immutable TTL
5. `/*` to the function, short TTL

Once CloudFront fronts the function, the Function URL stops being the public
entry point and should be restricted.

## Open questions

- Domain name and registrar.
- Whether `/projects/{slug}` is worth having before there are projects beyond
  this one.
- Fresh portrait, or the existing eight-year-old one as a stopgap.
