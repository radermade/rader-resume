//! The single HTTP binary: public site, read API, and flow endpoints.
//!
//! Routing lives here and nowhere else. One function serving many routes beats
//! a function per route: fewer cold starts, one deploy, one place for auth.

use std::sync::LazyLock;

use axum::{Json, Router, response::Html, routing::get};
use lambda_http::{Error, run};
use maud::{DOCTYPE, Markup, html};
use resume_core::Resume;

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

/// Parsed once at cold start. A parse failure here is a build-time content
/// error that validation should have caught, so failing fast is correct.
static RESUME: LazyLock<Resume> =
    LazyLock::new(|| resume_core::load().expect("embedded content must be valid"));

fn router() -> Router {
    Router::new()
        .route("/", get(home))
        .route("/api/resume", get(api_resume))
        .route("/healthz", get(healthz))
}

async fn healthz() -> &'static str {
    "ok"
}

async fn api_resume() -> Json<&'static Resume> {
    Json(&RESUME)
}

async fn home() -> Html<String> {
    // Rendering moves into resume-core once the templates grow past this.
    Html(page(&RESUME).into_string())
}

fn page(resume: &Resume) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" data-theme="dark" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (resume.profile.full_name) }
            }
            body {
                main {
                    h1 { "< " (resume.profile.preferred_name) " >" }
                    p { (resume.profile.headline) }

                    h2 { "< Experience >" }
                    ul {
                        @for role in &resume.roles {
                            li {
                                span { (role.employer) }
                                " — "
                                span { (role.start) " to " (role.end) }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_renders_a_document() {
        let rendered = page(&RESUME).into_string();
        assert!(rendered.starts_with("<!DOCTYPE html>"));
        assert!(rendered.contains("data-theme=\"dark\""));
        assert!(rendered.contains("National Insurance Producer Registry"));
    }
}
