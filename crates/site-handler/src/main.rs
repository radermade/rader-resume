//! The single HTTP binary: public site, read API, and the provenance check.
//!
//! Routing lives here and nowhere else. Rendering lives in `resume-core`, so
//! the same functions serve requests now and produce artifacts in the publish
//! pipeline later.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use axum::{
    Form, Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use check_runner::Checker;
use lambda_http::{Error, run};
use resume_core::provenance::{self, Check, CheckRun, RunStatus};
use resume_core::taxonomy::{self, Taxonomy};
use resume_core::{Dimension, FitReport, ImpreciseDate, PageContext, Provenance, Resume};
use run_store::Runs;
use serde::{Deserialize, Serialize};

/// Commit SHA this binary was built from. CI supplies it; local builds get
/// `"dev"`.
const GIT_SHA: &str = match option_env!("GIT_SHA") {
    Some(sha) => sha,
    None => "dev",
};

/// Reference month for derived values, as `YYYY-MM`. Set at build time so
/// `resume-core` stays clock-free and its tests stay deterministic.
const BUILD_MONTH: &str = match option_env!("BUILD_MONTH") {
    Some(month) => month,
    None => "2026-01",
};

/// Canonical origin. Absent until a domain exists, in which case canonical and
/// Open Graph URL tags are omitted rather than guessed.
const SITE_URL: Option<&str> = option_env!("SITE_URL");

/// Where provenance checks look for this site's own artifacts.
const LOCAL_ORIGIN: &str = "http://127.0.0.1:3000";

static RESUME: LazyLock<Resume> =
    LazyLock::new(|| resume_core::load().expect("embedded content must be valid"));

static CONTENT_ETAG: LazyLock<String> = LazyLock::new(|| {
    format!(
        "\"{}\"",
        resume_core::fingerprint(resume_core::CONTENT.as_bytes())
    )
});

static SCHEMA: LazyLock<serde_json::Value> = LazyLock::new(resume_core::content_schema);

static SCHEMA_ETAG: LazyLock<String> = LazyLock::new(|| {
    format!(
        "\"{}\"",
        resume_core::fingerprint(SCHEMA.to_string().as_bytes())
    )
});

static AS_OF: LazyLock<ImpreciseDate> = LazyLock::new(|| {
    BUILD_MONTH
        .parse()
        .expect("BUILD_MONTH must be YYYY or YYYY-MM")
});

/// Deterministic, so it is computed once rather than per request.
static PLAN: LazyLock<Vec<Check>> = LazyLock::new(|| provenance::plan(&RESUME, short_sha()));

static TAXONOMY: LazyLock<Taxonomy> =
    LazyLock::new(|| Taxonomy::load().expect("embedded taxonomy must be valid"));

static PAGE: LazyLock<String> =
    LazyLock::new(|| resume_core::page(&RESUME, page_context()).into_string());

static PAGE_ETAG: LazyLock<String> =
    LazyLock::new(|| format!("\"{}\"", resume_core::fingerprint(PAGE.as_bytes())));

/// Everything on `/stack` is embedded at build time, so the page is rendered
/// once and served from memory like the home page. Nothing about it varies per
/// request, which is the property that lets Phase 4 publish it as a static
/// artifact and stop invoking a function for it at all.
static STACK: LazyLock<String> = LazyLock::new(|| {
    let architecture =
        resume_core::architecture::load().expect("embedded architecture must be valid");
    resume_core::stack_page(&RESUME, page_context(), &architecture).into_string()
});

static STACK_ETAG: LazyLock<String> =
    LazyLock::new(|| format!("\"{}\"", resume_core::fingerprint(STACK.as_bytes())));

/// Whether the provenance check has a working execution path here.
///
/// True with the state machine configured, or locally where a long-lived
/// process makes the in-process runner viable. False in Lambda without the
/// state machine, because a `tokio::spawn` run does not survive the execution
/// environment freezing after a response returns, and an in-process store is
/// per-instance so a poll may hit a different one.
///
/// A `OnceLock` rather than state threaded through every handler, because it
/// is set once at startup and never changes. Defaults to disabled if somehow
/// read first, which is the safe direction.
static PROVENANCE_ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// The lookup is split from the predicate so the interesting half is testable
/// without mutating the process environment, which is `unsafe` in Rust 2024
/// and forbidden at the workspace root.
fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.is_empty())
}

fn non_empty_env(name: &str) -> Option<String> {
    non_empty(std::env::var(name).ok())
}

fn provenance_enabled() -> bool {
    *PROVENANCE_ENABLED.get().unwrap_or(&false)
}

/// Reports already produced, keyed by job description fingerprint.
///
/// Express workflows do not deduplicate by execution name the way Standard
/// ones do, so the collision the provenance feature gets free from the service
/// has to live here instead. Resubmitting the same posting, or reloading after
/// a form post, then costs nothing.
///
/// Per-instance, and deliberately not promoted to DynamoDB. A miss is a
/// recomputation of deterministic work measured in microseconds, so the
/// failure mode of a cold instance is invisible, and a visitor's pasted text
/// is not something to persist for a performance gain nobody would notice.
///
/// Analysis is pure: the same text against the same embedded content and
/// taxonomy always yields the same report. Nothing can change within an
/// instance's lifetime that would make a cached entry wrong, which is what
/// makes caching safe without a TTL. The `generatedAt` stamp reflects when the
/// report was computed, which stays true.
static FIT_CACHE: LazyLock<std::sync::Mutex<HashMap<String, FitReport>>> =
    LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

/// Cache ceiling. Low on purpose: this is a courtesy for repeat submissions,
/// not a store, and an unauthenticated endpoint should not let a stream of
/// distinct postings grow a warm instance's memory without bound. Full, it
/// clears rather than evicting cleverly, because tracking recency to protect
/// microseconds of arithmetic would be the expensive part.
const FIT_CACHE_LIMIT: usize = 32;

fn cached_report(fingerprint: &str) -> Option<FitReport> {
    lock_fit_cache().get(fingerprint).cloned()
}

fn cache_report(fingerprint: &str, report: &FitReport) {
    let mut cache = lock_fit_cache();

    if cache.len() >= FIT_CACHE_LIMIT {
        cache.clear();
    }

    cache.insert(fingerprint.to_owned(), report.clone());
}

/// Poisoning means a holder panicked mid-insert. The worst case is a stale or
/// empty cache, so recovering beats propagating a panic into a request that
/// could still be answered.
fn lock_fit_cache() -> std::sync::MutexGuard<'static, HashMap<String, FitReport>> {
    FIT_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// How the fit analysis runs.
///
/// `Inline` is the local path and needs no AWS at all. `Workflow` invokes the
/// Express state machine synchronously. Both call the same `resume-core`
/// functions, so they cannot produce different answers.
#[derive(Clone)]
enum FitExecutor {
    Inline,
    Workflow {
        client: aws_sdk_sfn::Client,
        arn: String,
    },
}

/// How a provenance run executes.
///
/// `Local` spawns a sequential in-process runner, which only works in a
/// single long-lived process. `Workflow` starts the Standard state machine,
/// which is the only path that works in Lambda.
#[derive(Clone)]
enum ProvenanceExecutor {
    Local,
    Workflow {
        client: aws_sdk_sfn::Client,
        arn: String,
    },
}

#[derive(Clone)]
struct AppState {
    runs: Runs,
    checker: Arc<Checker>,
    fit: FitExecutor,
    provenance: ProvenanceExecutor,
}

#[derive(Serialize)]
struct Build {
    commit: String,
    schema_version: u32,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // The runtime API variable is only present inside Lambda, which makes it a
    // reliable discriminator and avoids a feature flag or a second binary.
    let in_lambda = std::env::var_os("AWS_LAMBDA_RUNTIME_API").is_some();

    init_tracing(in_lambda);

    let fit_arn = non_empty_env("FIT_STATE_MACHINE_ARN");
    let provenance_arn = non_empty_env("PROVENANCE_STATE_MACHINE_ARN");

    // One client, built only if something needs it, so local development makes
    // no credential lookup at all.
    let sfn = if fit_arn.is_some() || provenance_arn.is_some() {
        let config = aws_config::load_from_env().await;
        Some(aws_sdk_sfn::Client::new(&config))
    } else {
        None
    };

    let fit = match (fit_arn, sfn.clone()) {
        (Some(arn), Some(client)) => {
            tracing::info!("fit analysis will run through the workflow");
            FitExecutor::Workflow { client, arn }
        }
        _ => {
            tracing::info!("fit analysis will run inline");
            FitExecutor::Inline
        }
    };

    let provenance = match (provenance_arn, sfn) {
        (Some(arn), Some(client)) => ProvenanceExecutor::Workflow { client, arn },
        _ => ProvenanceExecutor::Local,
    };

    // Enabled where there is a path that actually works: the state machine
    // anywhere, or the in-process runner in a long-lived local process. In
    // Lambda without a state machine it stays off, because a `tokio::spawn`
    // run does not survive the environment freezing after a response.
    let enabled = matches!(provenance, ProvenanceExecutor::Workflow { .. }) || !in_lambda;
    let _ = PROVENANCE_ENABLED.set(enabled);

    tracing::info!(
        provenance_enabled = enabled,
        workflow = matches!(provenance, ProvenanceExecutor::Workflow { .. }),
        "provenance configuration"
    );

    // DynamoDB where a table is configured, in-process otherwise. The
    // in-process variant is only correct in a single process, which is half of
    // why the feature is gated in Lambda.
    let runs = match std::env::var("PROVENANCE_TABLE") {
        Ok(table) if !table.is_empty() => Runs::dynamo(table).await,
        _ => Runs::memory(),
    };

    let origin = SITE_URL.unwrap_or(LOCAL_ORIGIN);
    let state = AppState {
        runs,
        checker: Arc::new(Checker::new(origin).map_err(Box::new)?),
        fit,
        provenance,
    };

    if in_lambda {
        run(router(state)).await
    } else {
        serve_locally(router(state)).await
    }
}

fn init_tracing(in_lambda: bool) {
    let builder = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO);

    if in_lambda {
        // JSON, no ANSI, no timestamps. CloudWatch supplies the timestamp, and
        // duplicating it just makes log queries worse.
        builder.json().with_ansi(false).without_time().init();
    } else {
        builder.init();
    }
}

/// Serves the same `Router` over plain HTTP for local development.
async fn serve_locally(app: Router) -> Result<(), Error> {
    let addr = std::env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("listening on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

fn router(state: AppState) -> Router {
    let mut router = Router::new()
        .route("/", get(home))
        .route("/stack", get(stack))
        .route("/api/resume", get(api_resume))
        .route("/api/projects", get(api_projects))
        .route("/api/schema", get(api_schema))
        .route("/api/build", get(api_build))
        .route("/fit", get(fit_form).post(analyse_fit))
        // Fragment, for the dialog on the resume. POST only: there is nothing
        // to GET here, and a bare fragment is not a page.
        .route("/fit/fragment", axum::routing::post(analyse_fit_fragment))
        .route("/robots.txt", get(robots))
        .route("/sitemap.xml", get(sitemap))
        // Content-hashed embedded assets (the portrait). The filename carries
        // the hash, so the lookup is by full path and the cache is immutable.
        .route("/assets/{file}", get(asset))
        .route("/healthz", get(healthz));

    // Registered only where the run store actually works. See
    // `PROVENANCE_ENABLED`.
    if provenance_enabled() {
        router = router
            .route("/provenance", get(provenance_plan).post(start_provenance))
            .route("/provenance/{id}", get(provenance_run));
    }

    router.with_state(state)
}

async fn healthz() -> &'static str {
    "ok"
}

/// Serves an embedded, content-hashed binary asset (the portrait).
///
/// The URL carries an FNV hash of the bytes, so a match is served with a long
/// immutable cache and a miss is a clean 404 rather than a stale serve. The
/// bytes and the URL both come from `resume_core::assets`, so the renderer and
/// this handler cannot disagree about a name.
async fn asset(Path(file): Path<String>) -> Response {
    let path = format!("/assets/{file}");
    match resume_core::assets::by_url(&path) {
        Some(asset) => (
            [
                (header::CONTENT_TYPE, asset.content_type),
                (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
            ],
            asset.bytes,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn stack(headers: HeaderMap) -> Response {
    cached(&headers, &STACK_ETAG, Html(STACK.clone()))
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

/// What the provenance live-commit check reads. Deliberately uncached: a stale
/// answer here would make the check meaningless.
async fn api_build() -> Response {
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(Build {
            commit: short_sha().to_owned(),
            schema_version: resume_core::SCHEMA_VERSION,
        }),
    )
        .into_response()
}

async fn fit_form() -> Html<String> {
    Html(resume_core::fit_form(&RESUME, page_context()).into_string())
}

#[derive(Deserialize)]
struct FitSubmission {
    description: String,
}

/// The page endpoint. Returns a full document, and is the path a visitor with
/// scripting disabled takes.
async fn analyse_fit(
    State(state): State<AppState>,
    Form(submission): Form<FitSubmission>,
) -> Response {
    match analyse(&state, &submission.description).await {
        Ok(report) => fit_response(Html(
            resume_core::fit_report(&RESUME, page_context(), &report).into_string(),
        )),
        Err(failure) => failure.into_response(),
    }
}

/// The fragment endpoint. Returns the report markup alone, for the dialog on
/// the resume to swap in.
///
/// A separate route rather than negotiating on an `Accept` header inside
/// `analyse_fit`: fragment endpoints return fragments and page endpoints
/// return documents, and collapsing the two is how that distinction rots.
async fn analyse_fit_fragment(
    State(state): State<AppState>,
    Form(submission): Form<FitSubmission>,
) -> Response {
    match analyse(&state, &submission.description).await {
        Ok(report) => fit_response(Html(
            resume_core::fit_report_fragment(&RESUME, &report).into_string(),
        )),
        Err(failure) => failure.into_response(),
    }
}

/// Why a fit analysis produced no report.
///
/// A small typed error rather than a ready `Response`. An axum `Response` in
/// an `Err` variant is large enough to be worth avoiding, and each surface
/// should own its own rendering rather than being handed finished bytes.
enum FitFailure {
    /// The submission itself: over the cap, or empty. The visitor's input
    /// rather than a server fault, and safe to repeat back.
    Rejected(String),
    /// A report that failed the citation guard. The message is deliberately
    /// opaque: the detail belongs in the log, not in a response.
    Unusable,
}

impl IntoResponse for FitFailure {
    fn into_response(self) -> Response {
        match self {
            Self::Rejected(message) => (StatusCode::BAD_REQUEST, message).into_response(),
            Self::Unusable => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "could not produce a report",
            )
                .into_response(),
        }
    }
}

/// Everything both fit endpoints do, so neither can answer differently.
async fn analyse(state: &AppState, description: &str) -> Result<FitReport, FitFailure> {
    let job = match resume_core::JobDescription::new(description) {
        Ok(job) => job,
        // The cap and the empty check are the only rejections, and both are
        // the visitor's input rather than a server fault.
        Err(error) => return Err(FitFailure::Rejected(error.to_string())),
    };

    // A repeat submission, or a reload after a form post, skips the workflow
    // entirely. Express gives no execution-name collision, so this is where
    // that happens or not at all.
    if let Some(report) = cached_report(job.fingerprint()) {
        tracing::info!(fingerprint = %job.fingerprint(), "serving cached report");
        return Ok(report);
    }

    // Workflow first where configured, inline as the fallback.
    //
    // Falling back rather than erroring is a deliberate tradeoff: the inline
    // path runs identical logic, so a visitor gets a correct answer either
    // way, and a broken workflow should not take the feature down. The cost is
    // that breakage is invisible without an alarm on failed executions, which
    // is why that alarm is a tracked task and not optional.
    let report = match &state.fit {
        FitExecutor::Workflow { client, arn } => match run_workflow(client, arn, &job).await {
            Ok(report) => report,
            Err(error) => {
                tracing::error!(%error, "fit workflow failed, falling back to inline");
                run_inline(&job)
            }
        },
        FitExecutor::Inline => run_inline(&job),
    };

    // A fabricated citation cannot reach a page. Nothing in either path
    // produces one, and the guarantee should not depend on that staying true.
    if let Err(error) = report.validate(&RESUME) {
        tracing::error!(%error, "report failed validation");
        return Err(FitFailure::Unusable);
    }

    // Stored only after validation, so a rejected report is never served from
    // the cache on a later request.
    cache_report(job.fingerprint(), &report);

    Ok(report)
}

/// Wraps either fit surface with the one header both need.
///
/// `no-store` regardless of the fingerprint cache. That cache is a
/// server-side courtesy keyed by the submitted text; the response still
/// contains one visitor's input and must not sit in a shared cache or a
/// browser history entry.
fn fit_response(body: Html<String>) -> Response {
    ([(header::CACHE_CONTROL, "no-store")], body).into_response()
}

/// The local stand-in for the workflow's `Parallel` state.
fn run_inline(job: &resume_core::JobDescription) -> FitReport {
    let extraction = taxonomy::extract(&TAXONOMY, job);
    let requirements = extraction.requirements;

    let branches = Dimension::ALL
        .iter()
        .map(|dimension| resume_core::match_dimension(*dimension, &requirements, &RESUME, *AS_OF))
        .collect();

    resume_core::aggregate(
        requirements,
        branches,
        Provenance {
            method: resume_core::fit::DETERMINISTIC_METHOD.to_owned(),
            revision: TAXONOMY.revision.clone(),
            generated_at: generated_at(),
        },
        extraction.unmatched_terms,
    )
}

async fn run_workflow(
    client: &aws_sdk_sfn::Client,
    arn: &str,
    job: &resume_core::JobDescription,
) -> Result<FitReport, Error> {
    let input = serde_json::json!({
        "description": job.text(),
        "asOf": AS_OF.to_string(),
        "generatedAt": generated_at(),
    })
    .to_string();

    let response = client
        .start_sync_execution()
        .state_machine_arn(arn)
        .input(input)
        .send()
        .await
        .map_err(Box::new)?;

    // An Express sync execution reports failure in the response rather than as
    // an API error, so the status has to be checked explicitly.
    if response.status() != &aws_sdk_sfn::types::SyncExecutionStatus::Succeeded {
        return Err(format!("execution status {:?}", response.status()).into());
    }

    let output = response
        .output()
        .ok_or_else(|| Error::from("execution produced no output"))?;

    Ok(serde_json::from_str(output)?)
}

/// Epoch seconds. Formatting a human-readable UTC timestamp needs date
/// arithmetic or a dependency, and nothing displays this yet.
fn generated_at() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs().to_string())
        .unwrap_or_else(|_| "unknown".to_owned())
}

async fn provenance_plan() -> Html<String> {
    Html(resume_core::provenance_plan(&RESUME, page_context(), &PLAN).into_string())
}

async fn start_provenance(State(state): State<AppState>) -> Response {
    let id = provenance::run_id(&PLAN, short_sha());

    // Claim it, and only start work if this caller won. A second click lands
    // on the existing run instead of duplicating a fan-out at third parties.
    match state.runs.claim(&CheckRun::queued(&id, PLAN.len())).await {
        Ok(true) => {
            if let Err(error) = begin_run(&state, &id).await {
                tracing::error!(%error, run = %id, "could not begin a run");
                return (StatusCode::SERVICE_UNAVAILABLE, "could not start a check")
                    .into_response();
            }
        }
        Ok(false) => tracing::info!(run = %id, "reusing an existing run"),
        Err(error) => {
            tracing::error!(%error, "could not claim a run");
            return (StatusCode::SERVICE_UNAVAILABLE, "could not start a check").into_response();
        }
    }

    // 303 so a refresh of the result page does not repost the form.
    Redirect::to(&format!("/provenance/{id}")).into_response()
}

async fn provenance_run(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.runs.get(&id).await {
        Ok(Some(run)) => {
            let body = resume_core::provenance_run(&RESUME, page_context(), &run, &PLAN);
            // Per-visitor, in-flight state. Caching it would serve one
            // visitor's run to another.
            (
                [(header::CACHE_CONTROL, "no-store")],
                Html(body.into_string()),
            )
                .into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "no such run").into_response(),
        Err(error) => {
            tracing::error!(%error, run = %id, "could not read a run");
            (StatusCode::SERVICE_UNAVAILABLE, "could not read the check").into_response()
        }
    }
}

/// Runs the checks in the background, updating the projection as it goes.
///
/// This is the local stand-in for the Step Functions `Map` state. It is
/// sequential rather than concurrent on purpose: the deployed version bounds
/// `MaxConcurrency` for the same reason, which is that this fans out at third
/// parties who did not opt in.
async fn begin_run(state: &AppState, id: &str) -> Result<(), Error> {
    match &state.provenance {
        ProvenanceExecutor::Workflow { client, arn } => {
            let input = serde_json::json!({
                "runId": id,
                "checks": &*PLAN,
            })
            .to_string();

            let result = client
                .start_execution()
                .state_machine_arn(arn)
                .name(id)
                .input(input)
                .send()
                .await;

            match result {
                Ok(_) => Ok(()),
                // The execution name is the run id, so a duplicate trigger
                // that slipped past the conditional write collides here
                // instead. Two layers of the same guarantee, both cheap.
                Err(error) if is_already_exists(&error) => {
                    tracing::info!(run = %id, "execution already running");
                    Ok(())
                }
                Err(error) => Err(Box::new(error).into()),
            }
        }
        ProvenanceExecutor::Local => {
            spawn_run(state.clone(), id.to_owned());
            Ok(())
        }
    }
}

fn is_already_exists(
    error: &aws_sdk_sfn::error::SdkError<
        aws_sdk_sfn::operation::start_execution::StartExecutionError,
    >,
) -> bool {
    matches!(
        error,
        aws_sdk_sfn::error::SdkError::ServiceError(service)
            if service.err().is_execution_already_exists()
    )
}

/// The local stand-in for the workflow's `Map` state.
///
/// Sequential rather than concurrent, matching the deployed
/// `maxConcurrency`, because both fan out at third parties who did not opt in.
fn spawn_run(state: AppState, id: String) {
    tokio::spawn(async move {
        // A failed write loses progress, which costs a visitor a re-click. Not
        // worth aborting the work for, so it is logged and carried on from.
        if let Err(error) = state.runs.set_status(&id, RunStatus::Running, None).await {
            tracing::error!(%error, "could not mark the run as running");
        }

        for check in PLAN.iter() {
            let result = state.checker.run(check).await;
            tracing::info!(
                check = %check.id,
                duration_ms = result.duration_ms,
                "check complete"
            );

            if let Err(error) = state.runs.append_result(&id, &result).await {
                tracing::error!(%error, check = %check.id, "could not record a result");
            }
        }

        if let Err(error) = state.runs.set_status(&id, RunStatus::Complete, None).await {
            tracing::error!(%error, "could not mark the run as complete");
        }
    });
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

fn page_context() -> PageContext<'static> {
    PageContext {
        commit: short_sha(),
        as_of: *AS_OF,
        site_url: SITE_URL,
        provenance_enabled: provenance_enabled(),
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

    #[test]
    fn taxonomy_loads_at_startup() {
        assert!(!TAXONOMY.entries.is_empty());
    }

    #[test]
    fn provenance_defaults_to_disabled() {
        // Tests never call main, so the flag is unset here. The default must
        // hide the feature rather than advertise a broken one.
        assert!(!provenance_enabled());
    }

    #[test]
    fn empty_env_vars_are_treated_as_absent() {
        // CDK omits the variable entirely without a domain, but an empty
        // string set by hand must not be mistaken for a configured ARN.
        assert!(non_empty(Some(String::new())).is_none());
        assert!(non_empty(None).is_none());
        assert_eq!(
            non_empty(Some("arn:aws:states:::sm".to_owned())).as_deref(),
            Some("arn:aws:states:::sm"),
        );
    }

    #[test]
    fn the_fit_cache_round_trips_and_stays_bounded() {
        let job = resume_core::JobDescription::new("Requirements: TypeScript, React.").unwrap();
        let report = run_inline(&job);

        // The cache is process-global and tests share a process. Starting from
        // a known state keeps this from depending on execution order, which is
        // the kind of thing that passes now and fails on an unrelated commit.
        lock_fit_cache().clear();

        assert!(cached_report(job.fingerprint()).is_none());
        cache_report(job.fingerprint(), &report);

        let hit = cached_report(job.fingerprint()).expect("a stored report is retrievable");
        assert_eq!(hit.score(), report.score());
        assert_eq!(hit.requirements.len(), report.requirements.len());

        // An unauthenticated endpoint must not let distinct submissions grow a
        // warm instance without bound.
        for i in 0..FIT_CACHE_LIMIT * 2 {
            cache_report(&format!("filler-{i}"), &report);
            assert!(lock_fit_cache().len() <= FIT_CACHE_LIMIT);
        }
    }

    #[test]
    fn identical_pastes_share_a_cache_entry() {
        // Normalization is what makes this work: two pastes differing only in
        // line wrapping fingerprint the same, so the second is a hit.
        let a = resume_core::JobDescription::new("Requirements:  Rust\n\nand AWS").unwrap();
        let b = resume_core::JobDescription::new("Requirements: Rust and AWS").unwrap();

        assert_eq!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn plan_is_non_empty_and_bounded() {
        assert!(!PLAN.is_empty());
        assert!(PLAN.len() <= provenance::MAX_CHECKS);
    }

    // Claim semantics are tested in `run-store`, where both variants live.
}
