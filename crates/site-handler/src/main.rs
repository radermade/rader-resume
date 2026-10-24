//! The single HTTP binary: public site, read API, and later the fit endpoints.
//!
//! Routing lives here and nowhere else. Rendering lives in `resume-core`, so
//! the same functions serve requests now and produce artifacts in the publish
//! pipeline later.

use std::sync::LazyLock;

use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use lambda_http::{Error, run};
use resume_core::{ImpreciseDate, PageContext, Resume};

/// Commit SHA this binary was built from. CI supplies it; local builds get
/// `"dev"`.
const GIT_SHA: &str = match option_env!("GIT_SHA") {
    Some(sha) => sha,
    None => "dev",
};

/// Reference month for derived values, as `YYYY-MM`. Set at build time so
/// `resume-core` stays clock-free and its tests stay deterministic. The site
/// is rebuilt on every publish, so build time is close enough to now.
const BUILD_MONTH: &str = match option_env!("BUILD_MONTH") {
    Some(month) => month,
    None => "2026-01",
};

/// Parsed once at cold start. A parse failure is a build-time content error
/// that validation should have caught, so failing fast is correct: the function
/// should not serve a half-understood document.
static RESUME: LazyLock<Resume> =
    LazyLock::new(|| resume_core::load().expect("embedded content must be valid"));

/// Derived from the content, not the commit, so a redeploy that changes no
/// content keeps caches warm.
static CONTENT_ETAG: LazyLock<String> = LazyLock::new(|| {
    format!(
        "\"{}\"",
        resume_core::fingerprint(resume_core::CONTENT.as_bytes())
    )
});

static SCHEMA: LazyLock<serde_json::Value> = LazyLock::new(resume_core::content_schema);

/// Separate from the content ETag on purpose: the schema changes with the
/// types, the content changes with the document, and neither implies the other.
static SCHEMA_ETAG: LazyLock<String> = LazyLock::new(|| {
    format!(
        "\"{}\"",
        resume_core::fingerprint(SCHEMA.to_string().as_bytes())
    )
});

/// Canonical origin. Absent until a domain exists, in which case canonical and
/// Open Graph URL tags are omitted rather than guessed.
const SITE_URL: Option<&str> = option_env!("SITE_URL");

static AS_OF: LazyLock<ImpreciseDate> = LazyLock::new(|| {
    BUILD_MONTH
        .parse()
        .expect("BUILD_MONTH must be YYYY or YYYY-MM")
});

fn page_context() -> PageContext<'static> {
    PageContext {
        commit: short_sha(),
        as_of: *AS_OF,
        site_url: SITE_URL,
    }
}

static PAGE: LazyLock<String> =
    LazyLock::new(|| resume_core::page(&RESUME, page_context()).into_string());

/// The rendered page changes with content, styles, or the commit, so it needs
/// its own validator rather than borrowing the content one.
static PAGE_ETAG: LazyLock<String> =
    LazyLock::new(|| format!("\"{}\"", resume_core::fingerprint(PAGE.as_bytes())));

#[tokio::main]
async fn main() -> Result<(), Error> {
    // JSON, no ANSI, no timestamps. CloudWatch supplies the timestamp, and
    // duplicating it just makes log queries worse.
    tracing_subscriber::fmt()
        .json()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .init();

    run(router()).await
}

fn router() -> Router {
    Router::new()
        .route("/", get(home))
        .route("/api/resume", get(api_resume))
        .route("/api/projects", get(api_projects))
        .route("/api/schema", get(api_schema))
        .route("/robots.txt", get(robots))
        .route("/sitemap.xml", get(sitemap))
        .route("/healthz", get(healthz))
}

async fn healthz() -> &'static str {
    "ok"
}

async fn home(headers: HeaderMap) -> Response {
    cached(&headers, &PAGE_ETAG, Html(PAGE.clone()))
}

async fn api_resume(headers: HeaderMap) -> Response {
    cached(&headers, &CONTENT_ETAG, Json(&*RESUME))
}

async fn api_projects(headers: HeaderMap) -> Response {
    cached(&headers, &CONTENT_ETAG, Json(&RESUME.projects))
}

async fn api_schema(headers: HeaderMap) -> Response {
    cached(&headers, &SCHEMA_ETAG, Json(&*SCHEMA))
}

async fn robots() -> Response {
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        resume_core::robots_txt(SITE_URL),
    )
        .into_response()
}

/// 404 without a configured origin: sitemap entries must be absolute URLs, so
/// serving a relative one would be invalid rather than merely unhelpful.
async fn sitemap() -> Response {
    match resume_core::sitemap_xml(SITE_URL) {
        Some(xml) => (
            [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
            xml,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn short_sha() -> &'static str {
    &GIT_SHA[..GIT_SHA.len().min(7)]
}

fn if_none_match(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value == "*" || value.split(',').any(|candidate| candidate.trim() == etag)
        })
}

fn cached(headers: &HeaderMap, etag: &str, body: impl IntoResponse) -> Response {
    if if_none_match(headers, etag) {
        return (StatusCode::NOT_MODIFIED, [(header::ETAG, etag)], ()).into_response();
    }

    (
        [
            (header::ETAG, etag),
            (header::CACHE_CONTROL, "public, max-age=60, must-revalidate"),
        ],
        body,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_is_rendered_once_and_carries_provenance() {
        assert!(PAGE.starts_with("<!DOCTYPE html>"));
        assert!(PAGE.contains(short_sha()));
    }

    #[test]
    fn etags_are_quoted_and_matched_exactly() {
        let etag = CONTENT_ETAG.as_str();
        assert!(etag.starts_with('"') && etag.ends_with('"'));

        let mut headers = HeaderMap::new();
        headers.insert(header::IF_NONE_MATCH, etag.parse().unwrap());
        assert!(if_none_match(&headers, etag));

        headers.insert(header::IF_NONE_MATCH, "\"nope\"".parse().unwrap());
        assert!(!if_none_match(&headers, etag));
    }

    #[test]
    fn each_resource_has_its_own_validator() {
        assert_ne!(CONTENT_ETAG.as_str(), SCHEMA_ETAG.as_str());
        assert_ne!(CONTENT_ETAG.as_str(), PAGE_ETAG.as_str());
        assert_ne!(SCHEMA_ETAG.as_str(), PAGE_ETAG.as_str());
    }
}
