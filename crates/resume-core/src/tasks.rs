//! Step Functions task payloads.
//!
//! Every input and output is a named type here rather than an ad-hoc
//! `serde_json::Value`, so the compiler enforces that the state writing a
//! payload and the state reading it agree.
//!
//! `deny_unknown_fields` is deliberately absent: orchestration layers add
//! fields, and strictness there turns a service change into an outage. That is
//! the opposite of the rule for authored content, where an unknown field means
//! a typo.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::fit::{Assessment, Dimension, Requirement};

/// One unit of fit analysis work.
///
/// A tagged enum rather than one binary per state: these are all pure compute
/// with no AWS permissions and identical resource needs, so separate artifacts
/// would be copies without benefit. Failure isolation comes from separate
/// invocations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "step", rename_all = "camelCase")]
pub enum FitTask {
    /// Job description text in, classified requirements out.
    Extract(ExtractInput),
    /// One `Parallel` branch: the requirements for a single dimension.
    Match(MatchInput),
    /// Branch outputs in, a report out.
    Aggregate(AggregateInput),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtractInput {
    /// Capped at `fit::MAX_INPUT_BYTES` before it ever reaches the workflow,
    /// which is also what keeps this well inside the 256 KB state payload
    /// limit. No S3 pointer needed at this size.
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtractOutput {
    pub requirements: Vec<Requirement>,
    /// Fingerprint of the normalized description, carried through so the
    /// report can be cached and a result traced back to its input without
    /// retaining the input itself.
    pub fingerprint: String,
    /// Required clauses the taxonomy could not place.
    ///
    /// Carried to `Aggregate` rather than recomputed there, because the
    /// description is deliberately dropped after extraction and the count
    /// cannot be derived from the requirements alone.
    #[serde(default)]
    pub unmatched_terms: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MatchInput {
    pub dimension: Dimension,
    pub requirements: Vec<Requirement>,
    /// Reference date for tenure arithmetic, supplied by the caller so the
    /// task has no clock and the result is reproducible.
    pub as_of: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MatchOutput {
    pub dimension: Dimension,
    pub assessments: Vec<Assessment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AggregateInput {
    pub requirements: Vec<Requirement>,
    /// Branch outputs, in whatever order the `Parallel` state returned them.
    /// Aggregation reorders to follow requirement order.
    pub branches: Vec<MatchOutput>,
    pub generated_at: String,
    /// Passed through from extraction so the report states its own coverage.
    #[serde(default)]
    pub unmatched_terms: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fit::RequirementKind;

    fn requirement() -> Requirement {
        Requirement {
            id: "typescript".to_owned(),
            text: "TypeScript".to_owned(),
            kind: RequirementKind::MustHave,
            dimension: Dimension::Tech,
            min_years: None,
        }
    }

    #[test]
    fn task_variants_round_trip_through_their_tag() {
        let task = FitTask::Match(MatchInput {
            dimension: Dimension::Seniority,
            requirements: vec![requirement()],
            as_of: "2026-10".to_owned(),
        });

        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains(r#""step":"match""#));
        assert!(json.contains(r#""dimension":"seniority""#));

        let parsed: FitTask = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, FitTask::Match(_)));
    }

    #[test]
    fn unknown_fields_are_tolerated_on_task_payloads() {
        // Orchestration layers add fields. Rejecting them would turn a service
        // change into an outage, which is why these types are not strict.
        let parsed: FitTask = serde_json::from_str(
            r#"{"step":"extract","description":"Requirements: Rust","traceId":"abc"}"#,
        )
        .expect("an added field must not break the task");

        assert!(matches!(parsed, FitTask::Extract(_)));
    }
}

/// One unit of provenance checking work.
///
/// Separate binaries would be justified here on permissions grounds alone
/// (every step writes to DynamoDB and `RunCheck` also makes outbound HTTP
/// calls), but the permissions happen to be identical across the steps, so one
/// binary dispatching on the tag is still the honest choice.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "step", rename_all = "camelCase")]
pub enum ProvenanceTask {
    /// Marks the run as in progress.
    Begin(RunRef),
    /// One `Map` branch: a single outbound check.
    RunCheck(RunCheckInput),
    /// Marks the run complete. Reached whether or not individual checks
    /// passed, because a failed check is a result rather than a failure.
    Finish(RunRef),
    /// The `Catch` target. Only reached when the run itself broke.
    Fail(FailInput),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunRef {
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunCheckInput {
    pub run_id: String,
    /// The whole check, not an index into a plan.
    ///
    /// Passing the check itself means the branch needs no shared state and no
    /// lookup, which is what lets `MaxConcurrency` be the only thing bounding
    /// the fan-out. The plan is small enough that this stays well inside the
    /// 256 KB payload limit.
    pub check: crate::provenance::Check,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FailInput {
    pub run_id: String,
    /// Shown to the visitor, so it must never contain raw error payloads that
    /// could carry internal detail.
    pub diagnostic: String,
}
