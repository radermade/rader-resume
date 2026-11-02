//! One Lambda serving every state in the fit analysis workflow.
//!
//! Dispatches on the payload tag. All six states are pure compute with no AWS
//! permissions and identical resource needs, so separate binaries would be
//! copies without benefit. Failure isolation and per-branch timing come from
//! separate invocations, which the state machine provides either way.
//!
//! Every function called here lives in `resume-core` and is the same function
//! the HTTP handler uses locally, so the deployed and local paths cannot drift.

use lambda_runtime::{Error, LambdaEvent, service_fn};
use resume_core::fit::{DETERMINISTIC_METHOD, JobDescription, Provenance};
use resume_core::tasks::{
    AggregateInput, ExtractInput, ExtractOutput, FitTask, MatchInput, MatchOutput,
};
use resume_core::taxonomy::{self, Taxonomy};
use resume_core::{ImpreciseDate, Resume};
use std::sync::LazyLock;

/// Parsed once at cold start and reused across warm invocations.
static RESUME: LazyLock<Resume> =
    LazyLock::new(|| resume_core::load().expect("embedded content must be valid"));

static TAXONOMY: LazyLock<Taxonomy> =
    LazyLock::new(|| Taxonomy::load().expect("embedded taxonomy must be valid"));

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .json()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .init();

    lambda_runtime::run(service_fn(handler)).await
}

async fn handler(event: LambdaEvent<FitTask>) -> Result<serde_json::Value, Error> {
    // Force initialization outside the match so a cold start pays for it once
    // regardless of which state arrives first.
    let resume = &*RESUME;

    match event.payload {
        FitTask::Extract(input) => Ok(serde_json::to_value(extract(input)?)?),
        FitTask::Match(input) => Ok(serde_json::to_value(match_dimension(input, resume)?)?),
        FitTask::Aggregate(input) => aggregate(input, resume),
    }
}

fn extract(input: ExtractInput) -> Result<ExtractOutput, Error> {
    // The cap is enforced here as well as at the edge. A task that trusts its
    // caller to have validated is a task that breaks when a second caller
    // appears.
    let job = JobDescription::new(&input.description).map_err(Box::new)?;
    let extraction = taxonomy::extract(&TAXONOMY, &job);

    tracing::info!(
        requirements = extraction.requirements.len(),
        unmatched = extraction.unmatched_terms,
        fingerprint = %job.fingerprint(),
        "extracted"
    );

    Ok(ExtractOutput {
        requirements: extraction.requirements,
        fingerprint: job.fingerprint().to_owned(),
        unmatched_terms: extraction.unmatched_terms,
    })
}

fn match_dimension(input: MatchInput, resume: &Resume) -> Result<MatchOutput, Error> {
    let as_of: ImpreciseDate = input.as_of.parse().map_err(Box::new)?;

    let assessments =
        resume_core::match_dimension(input.dimension, &input.requirements, resume, as_of);

    tracing::info!(
        dimension = ?input.dimension,
        assessed = assessments.len(),
        "matched"
    );

    Ok(MatchOutput {
        dimension: input.dimension,
        assessments,
    })
}

fn aggregate(input: AggregateInput, resume: &Resume) -> Result<serde_json::Value, Error> {
    let branches = input
        .branches
        .into_iter()
        .map(|branch| branch.assessments)
        .collect();

    let report = resume_core::aggregate(
        input.requirements,
        branches,
        Provenance {
            method: DETERMINISTIC_METHOD.to_owned(),
            revision: TAXONOMY.revision.clone(),
            generated_at: input.generated_at,
        },
        input.unmatched_terms,
    );

    // A fabricated citation cannot leave this task. Nothing in the
    // deterministic path produces one, and the guarantee should not depend on
    // that remaining true.
    report.validate(resume).map_err(Box::new)?;

    tracing::info!(
        score = report.score(),
        tier = ?report.tier(),
        gaps = report.gaps().count(),
        "aggregated"
    );

    Ok(serde_json::to_value(report)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use resume_core::Dimension;

    fn requirements() -> Vec<resume_core::Requirement> {
        extract(ExtractInput {
            description: "Requirements: TypeScript, React, accessibility, mentoring. \
                          Nice to have: Kubernetes."
                .to_owned(),
        })
        .unwrap()
        .requirements
    }

    #[test]
    fn extract_rejects_oversized_input() {
        let huge = "x".repeat(resume_core::fit::MAX_INPUT_BYTES + 1);
        assert!(extract(ExtractInput { description: huge }).is_err());
    }

    #[test]
    fn extract_produces_classified_requirements() {
        let found = requirements();
        assert!(!found.is_empty());
        assert!(found.iter().any(|r| r.dimension == Dimension::Tech));
    }

    #[test]
    fn the_whole_workflow_runs_end_to_end() {
        let resume = &*RESUME;
        let requirements = requirements();

        let branches: Vec<MatchOutput> = Dimension::ALL
            .iter()
            .map(|dimension| {
                match_dimension(
                    MatchInput {
                        dimension: *dimension,
                        requirements: requirements.clone(),
                        as_of: "2026-10".to_owned(),
                    },
                    resume,
                )
                .unwrap()
            })
            .collect();

        let report = aggregate(
            AggregateInput {
                requirements,
                branches,
                generated_at: "2026-10-24T00:00:00Z".to_owned(),
                unmatched_terms: 3,
            },
            resume,
        )
        .expect("aggregation must succeed");

        assert!(report.get("assessments").is_some());
        assert!(report.get("provenance").is_some());

        // Coverage survives the hop between states. The description is dropped
        // after extraction, so if this count is not carried it cannot be
        // recovered, and the report would silently claim full coverage.
        assert_eq!(
            report.get("unmatchedTerms").and_then(|v| v.as_u64()),
            Some(3)
        );
    }

    #[test]
    fn a_bad_as_of_is_an_error_not_a_guess() {
        assert!(
            match_dimension(
                MatchInput {
                    dimension: Dimension::Tech,
                    requirements: requirements(),
                    as_of: "not-a-date".to_owned(),
                },
                &RESUME,
            )
            .is_err()
        );
    }
}
