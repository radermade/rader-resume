---
inclusion: always
---

# Frontend and Routing

Decision: server-rendered HTML with `maud`, routed by `axum`, htmx for
interactivity. No WASM SPA.

Rationale: a resume site is read by recruiters on phones and by crawlers.
Server rendering wins on first paint and SEO, and collapses three routing
layers into one. The main argument for WASM is sharing `resume-core` types into
the browser, but rendering already happens where those types live.

## Where rendering happens

Amended after Phase 2. Rendering at request time and rendering at publish time
are both server rendering; the difference is when.

Content changes only on commit, so rendering identical HTML on every request is
waste. The target is static-first: the publish pipeline renders artifacts with
the same `resume-core` functions, CloudFront serves them from S3, and the
Lambda handles only what is genuinely dynamic. Reads cost zero invocations.

- Until Phase 4 exists, the handler renders on request from embedded content.
  That is a stepping stone, not the destination.
- Rendering functions take typed data and return `Markup`, with no AWS or HTTP
  types in their signatures. That is what makes the caller swappable.
- Nothing visitor-specific may depend on per-request rendering, or the static
  path becomes impossible. This is a hard constraint, and theming is where it
  bites.

## The three routing layers

Name the layer before changing routing behavior; the fix usually belongs in a
different one than it first appears.

- Edge routing: CloudFront behaviors deciding which origin serves a path.
  Static assets to S3, everything else to the function.
- Server routing: method plus path matched to a handler inside the Lambda. This
  is where essentially all routing logic belongs.
- Client routing: in-browser history manipulation. Does not exist in this
  project. If it ever does, it brings a deep-link fallback rewrite with it.

## Server routing

```rust
use axum::{routing::get, Router};
use lambda_http::{run, tracing, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let app = Router::new()
        .route("/", get(home))
        .route("/projects/{slug}", get(project))
        .route("/api/resume", get(resume_json))
        .route("/flow/{session}", get(flow_step).post(flow_advance))
        .with_state(AppState::init().await);

    run(app).await
}
```

- `lambda_http::run` takes the axum `Router` directly, since the Router is a
  tower Service. Do not hand-roll path matching on `lambda_http::Request`.
- Path params use axum 0.8 brace syntax, `{slug}` not `:slug`. Most examples
  online predate the change.
- The same `Router` serves under a plain hyper binding for local development.
  Routing bugs get found locally, not through a deploy cycle.
- Shared state is built once at cold start and passed via `with_state`, per the
  handler rules in the Rust standards.

## Rendering

- `maud` templates compile with the handler, so a malformed template or a
  renamed field is a build failure rather than a runtime 500.
- Rendering functions live in `resume-core` and return markup from typed data.
  Handlers wire requests to them; they do not contain markup.
- Snapshot test rendered output. HTML regressions are easy to miss by eye.
- Semantic markup, real landmarks, labelled form controls, visible focus, and
  working keyboard navigation. Accessibility is part of the definition of done,
  not an audit at the end.

## Interactivity

- htmx for the stepped flow. The server returns HTML fragments, htmx swaps
  them, routing stays server-side.
- Every interaction works without JavaScript. htmx enhances a form post that
  already functions on its own.
- Fragment endpoints return fragments, page endpoints return full documents.
  Keep them distinct rather than branching on a header deep inside a handler.

## Flow URLs

- Flow steps are addressable: `/flow/{session}`. A visitor who refreshes or
  reopens a link lands on their current step instead of restarting.
- The handler reads the step projection from the session record. It makes no
  Step Functions calls on a GET.
- Session IDs are opaque and unguessable. They identify a flow, they are not a
  credential for anything else, and they carry no personal data.
- An expired or unknown session renders a clean restart, not an error page.

## Caching

- HTML and API responses: short TTL with ETags, revalidated at the edge.
- Assets: content-hashed filenames with long immutable TTLs.
- Flow responses: no-store. Never cache a page containing per-visitor state.
- The published commit SHA appears in the page footer, so "is my change live"
  is a lookup rather than a guess.
