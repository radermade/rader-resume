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

## Theme, without foreclosing static

Open question 7 resolved to static-first reads, which rules out injecting a
per-visitor theme into HTML at request time. Start with the CSS-only approach:
dark by default, `prefers-color-scheme: light` override, no manual toggle. It
covers most visitors, costs nothing, and works identically whether the HTML
came from a Lambda or from S3.

If a persistent manual toggle turns out to matter, the escalation is a cookie
in the CloudFront cache key serving two cached variants, not a handler that
renders per visitor. A pure-CSS `:has()` toggle is the middle option and does
not persist across pages.

`POST /theme` and the `Vary: Cookie` question only exist in the cache-key
variant. Do not build them speculatively.

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

## DNS

Domain: `rader-resume.com`, registered at Squarespace, currently pointing at
the dead site.

Registration stays at Squarespace; DNS delegates to Route 53. The reason is
the apex: `rader-resume.com` cannot be a CNAME, and Squarespace DNS has no
alias record type that can target a CloudFront distribution. Route 53 alias
records can, and it puts DNS in IaC with everything else.

The hosted zone lives in the bootstrap app with a retain policy, not in the
application stack. Recreating a zone issues new nameservers, which means
re-entering them at the registrar and waiting out propagation again. The
application stack looks the zone up rather than owning it.

Cutover order matters, because each step blocks the next:

1. Deploy the hosted zone, read the nameservers from the stack output.
2. Set those as custom nameservers at Squarespace. Propagation is hours, not
   minutes.
3. Deploy the ACM certificate in `us-east-1`. DNS validation writes records
   into the hosted zone automatically, and fails if delegation has not landed.
4. Deploy CloudFront, test against the distribution domain directly.
5. Only then point the apex and `www` at the distribution.

Testing against the CloudFront domain before the DNS flip means a broken
deploy never takes the domain down with it.

## Open questions

- Whether `/projects/{slug}` is worth having before there are projects beyond
  this one.
- Fresh portrait, or the existing eight-year-old one as a stopgap.
