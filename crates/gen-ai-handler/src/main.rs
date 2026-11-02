//! The GenAI lab's HTTP binary. Every route under `/gen-ai`.
//!
//! Routing lives here and nowhere else. Rendering lives in `gen-ai-core`, so
//! the same functions serve a request now and could produce static artifacts
//! later.
//!
//! No AWS client, no model call, no store. A scenario is content embedded at
//! build time, so this binary needs no credentials to serve a page and no
//! session to remember a position.

use std::sync::LazyLock;

use axum::{
    Form, Router,
    extract::Path,
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use gen_ai_core::{Cursor, Scenario};
use lambda_http::{Error, run};
use serde::Deserialize;

/// Parsed once at cold start. A malformed document is a build-time bug that a
/// `gen-ai-core` test catches, so failing fast here is the correct behaviour.
static CATALOG: LazyLock<Vec<Scenario>> =
    LazyLock::new(|| gen_ai_core::catalog().expect("embedded scenarios must be valid"));

/// The landing never varies, so it is rendered once and served from memory.
static LANDING: LazyLock<String> = LazyLock::new(|| gen_ai_core::landing(&CATALOG).into_string());

static LANDING_ETAG: LazyLock<String> = LazyLock::new(|| etag(LANDING.as_bytes()));

/// A weak content fingerprint for conditional requests.
///
/// FNV-1a. Not a security primitive and not used as one: it only has to change
/// when the bytes change.
fn etag(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("\"{hash:016x}\"")
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let in_lambda = std::env::var_os("AWS_LAMBDA_RUNTIME_API").is_some();

    let builder = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO);
    if in_lambda {
        // CloudWatch supplies the timestamp; duplicating it only makes log
        // queries worse.
        builder.json().with_ansi(false).without_time().init();
    } else {
        builder.init();
    }

    // Force the catalog open before the first request, so a bad document is a
    // cold-start failure rather than a confusing 500.
    tracing::info!(scenarios = CATALOG.len(), "catalog loaded");

    let app = router();

    if in_lambda {
        run(app).await
    } else {
        serve_locally(app).await
    }
}

fn router() -> Router {
    Router::new()
        .route("/gen-ai", get(landing))
        .route("/gen-ai/{id}", get(scenario).post(advance))
        // The swap target for the nav script. POST, returning the lesson
        // fragment alone. Exact, so it is matched before `{id}` would take it.
        .route("/gen-ai/{id}/lesson", post(lesson_fragment))
        // Content-hashed, so the path carries the version and the whole hop
        // past this point is `{*rest}`. A wildcard captures the full suffix,
        // which a single `{param}` would not.
        .route("/gen-ai/assets/{*rest}", get(asset))
        .route("/gen-ai/healthz", get(|| async { "ok" }))
}

/// Serves a workbench asset, matched by its whole content-hashed path.
///
/// Immutable: the filename changes when the bytes do, so a far-future
/// `Cache-Control` never serves stale. A request for an old hash is a clean
/// 404 rather than a stale 200.
async fn asset(uri: axum::http::Uri) -> Response {
    if let Some(asset) = gen_ai_core::assets::by_url(uri.path()) {
        return (
            [
                (header::CONTENT_TYPE, asset.content_type),
                (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
            ],
            asset.body,
        )
            .into_response();
    }

    // Local-development fallback for the vendored Pyodide runtime.
    //
    // In production CloudFront routes `/gen-ai/assets/pyodide/*` to the S3
    // asset bucket, so these requests never reach the Lambda. Locally there is
    // no CloudFront, so the handler must serve the vendored files off disk or
    // the workbench cannot load its runtime. Guarded to the Lambda-absent case
    // so the deployed function never reads from a filesystem it does not have.
    if std::env::var_os("AWS_LAMBDA_RUNTIME_API").is_none() {
        if let Some(response) = serve_vendored_runtime(uri.path()).await {
            return response;
        }
    }

    StatusCode::NOT_FOUND.into_response()
}

/// Serves a vendored Pyodide file from disk, for local development only.
///
/// Path-traversal safe: the request tail is rejected if it contains `..` or an
/// absolute component, and the resolved path must stay inside the vendor
/// directory. Returns `None` for anything outside the pyodide prefix or not on
/// disk, which the caller turns into a 404.
async fn serve_vendored_runtime(path: &str) -> Option<Response> {
    const PREFIX: &str = "/gen-ai/assets/pyodide/";
    let tail = path.strip_prefix(PREFIX)?;

    // No traversal, no absolute paths, no empty segments.
    if tail.is_empty()
        || tail
            .split('/')
            .any(|seg| seg.is_empty() || seg == "." || seg == "..")
    {
        return None;
    }

    // The vendor directory, relative to the crate at dev time.
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/gen-ai/pyodide");
    let full = base.join(tail);

    // Defence in depth: the canonical path must stay under the vendor dir.
    // Synchronous std::fs is fine here: this path is local-development only,
    // never compiled into the deployed function's hot path, and the files are
    // read once and cached by the browser. Avoids pulling tokio's fs feature
    // in just for dev serving.
    let canonical_base = std::fs::canonicalize(&base).ok()?;
    let canonical_full = std::fs::canonicalize(&full).ok()?;
    if !canonical_full.starts_with(&canonical_base) {
        return None;
    }

    let bytes = std::fs::read(&canonical_full).ok()?;
    let content_type = content_type_for(tail);

    Some(
        (
            [
                (header::CONTENT_TYPE, content_type),
                (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
            ],
            bytes,
        )
            .into_response(),
    )
}

/// Minimal content-type mapping for the vendored runtime files.
fn content_type_for(name: &str) -> &'static str {
    match name.rsplit_once('.').map(|(_, ext)| ext) {
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("json") => "application/json; charset=utf-8",
        Some("whl") | Some("zip") => "application/octet-stream",
        Some("ts") => "application/octet-stream",
        _ => "application/octet-stream",
    }
}

async fn landing(headers: HeaderMap) -> Response {
    cached(&headers, &LANDING_ETAG, Html(LANDING.clone()))
}

/// Opening a project always starts at step 1.
///
/// Steps are not addressable, so there is no step parameter to honour here.
/// That is what lets phase 2's console survive Next: advancing is a swap, not
/// a navigation.
async fn scenario(Path(id): Path<String>) -> Response {
    let Some(scenario) = gen_ai_core::find(&CATALOG, &id) else {
        // An unknown project is a stale link, not an error worth a page.
        return Redirect::to("/gen-ai").into_response();
    };

    render(&Cursor::at(scenario, 0))
}

#[derive(Deserialize)]
struct Advance {
    /// The step the page was rendered at. Untrusted: it arrives in the body.
    #[serde(default)]
    step: usize,
    /// `previous` or `next`. Optional and defaulted, because a button only
    /// submits its own `name=value` when it is the one clicked: a disabled
    /// Previous on step 1, or an Enter-key submit, posts `step` with no `go`.
    /// A required field there turns that into a 400; defaulted, it holds
    /// position, which is the "anything else" branch below.
    #[serde(default)]
    go: String,
}

/// Resolves a scenario and the step the form asks to move to.
///
/// Shared by the full-page advance and the fragment swap, so the two cannot
/// move to different steps from the same input. `Cursor::at` clamps, so a
/// hand-edited index lands on the nearest real step rather than erroring, and
/// an absent or unrecognised `go` holds position rather than guessing.
fn moved<'a>(scenario: &'a gen_ai_core::Scenario, form: &Advance) -> Cursor<'a> {
    let cursor = Cursor::at(scenario, form.step);
    match form.go.as_str() {
        "next" => cursor.next(),
        "previous" => cursor.previous(),
        _ => cursor,
    }
}

async fn advance(Path(id): Path<String>, Form(form): Form<Advance>) -> Response {
    let Some(scenario) = gen_ai_core::find(&CATALOG, &id) else {
        return Redirect::to("/gen-ai").into_response();
    };

    render(&moved(scenario, &form))
}

/// The swap endpoint: the lesson region alone, for the nav script to drop into
/// `#lab-lesson`. A fragment, not a document, so it carries no shell and no
/// policy of its own; the script only swaps within one policy class, so the
/// document's CSP still governs.
async fn lesson_fragment(Path(id): Path<String>, Form(form): Form<Advance>) -> Response {
    let Some(scenario) = gen_ai_core::find(&CATALOG, &id) else {
        // No document to swap into; the script will fall back to navigation.
        return StatusCode::NOT_FOUND.into_response();
    };

    let cursor = moved(scenario, &form);
    let layout = match cursor.step() {
        gen_ai_core::Step::Overview { diagram, .. } => Some(gen_ai_core::diagram::layout(diagram)),
        gen_ai_core::Step::Call(_) => None,
    };

    (
        [(header::CACHE_CONTROL, "no-store")],
        Html(gen_ai_core::lesson(&cursor, layout.as_ref()).into_string()),
    )
        .into_response()
}

/// Renders a step.
///
/// Not cached. The response is derived from the posted step, and the policy it
/// carries varies with the diagram geometry on the page, so a shared cache
/// entry keyed only on the path would be wrong.
fn render(cursor: &Cursor<'_>) -> Response {
    (
        [(header::CACHE_CONTROL, "no-store")],
        Html(gen_ai_core::step_page(cursor).into_string()),
    )
        .into_response()
}

/// Serves a pre-rendered body, or a 304 when the client already has it.
fn cached(headers: &HeaderMap, tag: &str, body: Html<String>) -> Response {
    if if_none_match(headers, tag) {
        return (StatusCode::NOT_MODIFIED, [(header::ETAG, tag)]).into_response();
    }

    (
        [
            (header::ETAG, tag),
            (header::CACHE_CONTROL, "public, max-age=60, must-revalidate"),
        ],
        body,
    )
        .into_response()
}

/// Exact match only. A weak comparison would let a changed page serve stale.
fn if_none_match(headers: &HeaderMap, tag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.split(',').any(|candidate| candidate.trim() == tag))
}

/// The same `Router` over plain HTTP, so `cargo lambda watch` and a bare
/// `cargo run` both work with no credentials and no network.
async fn serve_locally(app: Router) -> Result<(), Error> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await?;
    tracing::info!("listening on http://127.0.0.1:3001/gen-ai");

    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalog_opens_at_cold_start() {
        assert!(!CATALOG.is_empty());
        assert!(LANDING.starts_with("<!DOCTYPE html>"));
    }

    #[test]
    fn etags_are_quoted_and_content_sensitive() {
        let tag = etag(b"one");

        assert!(tag.starts_with('"') && tag.ends_with('"'));
        assert_eq!(tag, etag(b"one"));
        assert_ne!(tag, etag(b"two"));
    }

    #[test]
    fn conditional_requests_match_exactly() {
        let tag = LANDING_ETAG.as_str();

        let mut headers = HeaderMap::new();
        headers.insert(header::IF_NONE_MATCH, tag.parse().unwrap());
        assert!(if_none_match(&headers, tag));

        headers.insert(header::IF_NONE_MATCH, "\"nope\"".parse().unwrap());
        assert!(!if_none_match(&headers, tag));
    }

    #[test]
    fn an_unknown_step_clamps_rather_than_failing() {
        let scenario = &CATALOG[0];

        // The index arrives in a POST body, so it is input. A value past the
        // end lands on the last step.
        let cursor = Cursor::at(scenario, 9_999);
        assert_eq!(cursor.index(), scenario.len() - 1);
        assert!(cursor.is_last());
    }

    #[test]
    fn an_absent_go_holds_position() {
        // Regression: a button only submits its own name=value when clicked,
        // so a disabled Previous on step 1 or an Enter-key submit posts step
        // with no go. That reaches `moved` as an empty go string, and the
        // visitor hit it by stepping back to the overview. It must hold, not
        // 400. (`#[serde(default)]` on the field is what lets `Form` build
        // this value at all; here we assert the arithmetic once it exists.)
        let scenario = &CATALOG[0];

        let no_go = Advance {
            step: 0,
            go: String::new(),
        };
        assert_eq!(
            moved(scenario, &no_go).index(),
            0,
            "an absent go holds position"
        );

        let mid = Advance {
            step: 1,
            go: String::new(),
        };
        let mid_index = Cursor::at(scenario, 1).index();
        assert_eq!(
            moved(scenario, &mid).index(),
            mid_index,
            "a go-less submit mid-scenario stays put rather than erroring"
        );
    }

    #[test]
    fn the_shared_mover_clamps_and_saturates() {
        let scenario = &CATALOG[0];

        // Both the full advance and the fragment swap go through `moved`, so
        // one test covers the step arithmetic for both routes.
        let past_end = Advance {
            step: 9_999,
            go: "next".into(),
        };
        assert_eq!(moved(scenario, &past_end).index(), scenario.len() - 1);

        let before_start = Advance {
            step: 0,
            go: "previous".into(),
        };
        assert_eq!(moved(scenario, &before_start).index(), 0);

        // A tampered action holds position rather than guessing.
        let nonsense = Advance {
            step: 0,
            go: "sideways".into(),
        };
        assert_eq!(moved(scenario, &nonsense).index(), 0);
    }

    #[test]
    fn navigation_saturates_at_both_ends() {
        let scenario = &CATALOG[0];

        let first = Cursor::at(scenario, 0);
        assert_eq!(first.previous().index(), 0);

        let last = Cursor::at(scenario, scenario.len() - 1);
        assert_eq!(last.next().index(), scenario.len() - 1);
    }

    #[test]
    fn an_unrecognised_action_holds_position() {
        let scenario = &CATALOG[0];
        let cursor = Cursor::at(scenario, 0);

        // Only `next` and `previous` move. A tampered value should not guess.
        assert_eq!(cursor.index(), 0);
    }
}
