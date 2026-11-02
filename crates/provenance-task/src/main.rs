//! One Lambda serving every state in the provenance check workflow.
//!
//! The check execution itself comes from `check-runner`, which the HTTP
//! handler also uses locally. The logic cannot drift between the two paths
//! because there is only one copy of it.

use std::sync::OnceLock;

use check_runner::Checker;
use lambda_runtime::{Error, LambdaEvent, service_fn};
use resume_core::provenance::RunStatus;
use resume_core::tasks::{FailInput, ProvenanceTask, RunCheckInput, RunRef};
use run_store::Runs;

/// Built once at cold start and reused across warm invocations.
static STATE: OnceLock<State> = OnceLock::new();

struct State {
    runs: Runs,
    checker: Checker,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .json()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .init();

    let table = std::env::var("PROVENANCE_TABLE")?;
    let origin = std::env::var("SITE_URL")?;

    let _ = STATE.set(State {
        runs: Runs::dynamo(table).await,
        checker: Checker::new(origin).map_err(Box::new)?,
    });

    lambda_runtime::run(service_fn(handler)).await
}

fn state() -> &'static State {
    STATE
        .get()
        .expect("state is initialized before the runtime starts")
}

async fn handler(event: LambdaEvent<ProvenanceTask>) -> Result<serde_json::Value, Error> {
    match event.payload {
        ProvenanceTask::Begin(input) => begin(input).await,
        ProvenanceTask::RunCheck(input) => run_check(input).await,
        ProvenanceTask::Finish(input) => finish(input).await,
        ProvenanceTask::Fail(input) => fail(input).await,
    }
}

async fn begin(input: RunRef) -> Result<serde_json::Value, Error> {
    state()
        .runs
        .set_status(&input.run_id, RunStatus::Running, None)
        .await
        .map_err(Box::new)?;

    Ok(serde_json::json!({ "runId": input.run_id }))
}

/// Runs one check and appends its result.
///
/// Returns success even when the check could not reach its target. An
/// unreachable host is a result, not a workflow failure, and the `Map` state's
/// tolerated failure percentage exists for the cases this cannot absorb.
async fn run_check(input: RunCheckInput) -> Result<serde_json::Value, Error> {
    let result = state().checker.run(&input.check).await;

    tracing::info!(
        run = %input.run_id,
        check = %input.check.id,
        duration_ms = result.duration_ms,
        outcome = ?result.outcome,
        "check complete"
    );

    // A write failure is worth failing the branch for: a result that is not
    // recorded is invisible to the visitor, and the retry is cheap.
    state()
        .runs
        .append_result(&input.run_id, &result)
        .await
        .map_err(Box::new)?;

    Ok(serde_json::json!({ "checkId": input.check.id }))
}

async fn finish(input: RunRef) -> Result<serde_json::Value, Error> {
    state()
        .runs
        .set_status(&input.run_id, RunStatus::Complete, None)
        .await
        .map_err(Box::new)?;

    tracing::info!(run = %input.run_id, "run complete");
    Ok(serde_json::json!({ "runId": input.run_id }))
}

/// Records a run-level failure.
///
/// Reached only when the workflow itself broke, not when checks failed. The
/// diagnostic is written as given, which is why the state machine supplies a
/// fixed string rather than passing the raw error through: a visitor-facing
/// field should not carry internal detail.
async fn fail(input: FailInput) -> Result<serde_json::Value, Error> {
    state()
        .runs
        .set_status(&input.run_id, RunStatus::Failed, Some(&input.diagnostic))
        .await
        .map_err(Box::new)?;

    tracing::error!(run = %input.run_id, diagnostic = %input.diagnostic, "run failed");
    Ok(serde_json::json!({ "runId": input.run_id }))
}
