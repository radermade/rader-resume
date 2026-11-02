//! Live provenance checking: independently verifying what is verifiable about
//! the published page.
//!
//! What this can establish: that external links resolve, that a referenced
//! GitHub repository exists and when it last changed, that published artifacts
//! are downloadable, and that the commit the footer claims is the commit being
//! served.
//!
//! What it cannot establish: employment, job titles, dates, or anything a
//! human asserted. Naming this "verification" without that caveat would be
//! overclaiming in a way that undermines the point of having it.
//!
//! No HTTP here. This module plans the work and interprets the results; the
//! task Lambdas make the calls. That keeps the planning logic testable without
//! a network.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Resume;

/// Minimum gap between fresh runs for the same plan.
///
/// A visitor-triggered fan-out of outbound requests is an amplifier. Repeated
/// clicks inside this window return the cached run rather than hammering third
/// parties who did not ask to be part of this.
pub const MIN_REFRESH_SECONDS: u64 = 15 * 60;

/// Upper bound on checks in a single run, so content growth cannot quietly
/// turn one click into a hundred outbound requests.
pub const MAX_CHECKS: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CheckKind {
    /// Any absolute URL in the content. Passes on a 2xx or 3xx.
    ///
    /// Repository URLs are checked as plain links. There was a `GitHubRepo`
    /// variant that called the API for the last push date; it was dropped
    /// because the unauthenticated limit is 60 requests an hour per source IP
    /// and Lambda egresses from shared addresses, so the budget was shared
    /// with strangers and the check would report "unreachable" for reasons
    /// having nothing to do with the repository. A link check answers the
    /// question the visitor actually asked.
    ExternalLink { url: String },
    /// A published artifact served by this site.
    Artifact { path: String },
    /// The commit the page claims, compared against what the site reports.
    LiveCommit { claimed: String },
}

/// One thing to check, derived from the content document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    /// Stable within a plan, so results can reference checks without
    /// repeating them.
    pub id: String,
    /// Shown to the visitor.
    pub label: String,
    /// Where in the document the claim lives, in the same path form
    /// `ValidationIssue` uses.
    pub source: String,
    pub kind: CheckKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum Outcome {
    Passed {
        detail: String,
    },
    /// Could not reach it. Explicitly not a failed claim: a corporate site
    /// blocking datacenter addresses says nothing about the employment.
    Unreachable {
        reason: String,
    },
    /// Reached it, and it disagreed with what the page claims. This is the
    /// only outcome that implies something is actually wrong.
    Mismatch {
        expected: String,
        found: String,
    },
    Skipped {
        reason: String,
    },
}

impl Outcome {
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }

    /// Whether this outcome indicates a genuine problem, as opposed to the
    /// internet being the internet.
    pub fn is_problem(&self) -> bool {
        matches!(self, Self::Mismatch { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    pub check_id: String,
    pub outcome: Outcome,
    pub duration_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RunStatus {
    Queued,
    Running,
    Complete,
    /// The run itself broke, as distinct from checks failing. Individual check
    /// failures are tolerated by design.
    Failed,
}

impl RunStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Running => "Checking",
            Self::Complete => "Complete",
            Self::Failed => "Could not complete",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Failed)
    }

    /// Stable wire form, for storage keys and attributes where a quoted JSON
    /// string would be awkward.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Complete => "complete",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "complete" => Some(Self::Complete),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// The projection a page render reads.
///
/// Each task writes into this as it completes. Reads never query execution
/// state: mid-flight output is invisible to `DescribeExecution`, and
/// `GetExecutionHistory` is not built for page-load traffic.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckRun {
    /// Fingerprint of the plan plus the live commit. Doubles as the Step
    /// Functions execution name, so duplicate triggers collide.
    pub id: String,
    pub status: RunStatus,
    pub planned: usize,
    #[serde(default)]
    pub results: Vec<CheckResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<String>,
}

impl CheckRun {
    pub fn queued(id: impl Into<String>, planned: usize) -> Self {
        Self {
            id: id.into(),
            status: RunStatus::Queued,
            planned,
            results: Vec::new(),
            diagnostic: None,
        }
    }

    pub fn progress(&self) -> u8 {
        if self.status.is_terminal() {
            return 100;
        }
        if self.planned == 0 {
            return 0;
        }
        let ratio = self.results.len() as f32 / self.planned as f32;
        (ratio * 100.0).round().min(99.0) as u8
    }

    pub fn passed(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.outcome.is_pass())
            .count()
    }

    pub fn problems(&self) -> impl Iterator<Item = &CheckResult> {
        self.results
            .iter()
            .filter(|result| result.outcome.is_problem())
    }

    /// One line for the visitor. Distinguishes "could not reach" from "wrong",
    /// because conflating them would be dishonest in both directions.
    pub fn summary(&self) -> String {
        let passed = self.passed();
        let problems = self.problems().count();
        let unreachable = self.results.len() - passed - problems;

        let mut text = format!("{passed} of {} checks passed", self.planned);
        if unreachable > 0 {
            text.push_str(&format!(", {unreachable} unreachable"));
        }
        if problems > 0 {
            text.push_str(&format!(", {problems} mismatched"));
        }
        text
    }
}

use crate::is_placeholder;

/// Builds the check plan from the content document.
///
/// Deterministic: the same content produces the same plan in the same order,
/// which is what makes the run fingerprint a usable cache key.
///
/// Placeholder URLs are skipped rather than checked, because `[github-url]` is
/// an unfilled blank, not a broken link.
pub fn plan(resume: &Resume, claimed_commit: &str) -> Vec<Check> {
    let mut checks = Vec::new();

    checks.push(Check {
        id: "live-commit".to_owned(),
        label: "Page reports the commit it was built from".to_owned(),
        source: "footer".to_owned(),
        kind: CheckKind::LiveCommit {
            claimed: claimed_commit.to_owned(),
        },
    });

    for (i, link) in resume.profile.links.iter().enumerate() {
        if is_placeholder(&link.url) {
            continue;
        }
        checks.push(Check {
            id: format!("profile-link-{i}"),
            label: format!("{} link resolves", link.label),
            source: format!("profile.links[{i}].url"),
            kind: CheckKind::ExternalLink {
                url: link.url.clone(),
            },
        });
    }

    for project in &resume.projects {
        if let Some(repo) = project.repo.as_deref().filter(|url| !is_placeholder(url)) {
            checks.push(Check {
                id: format!("project-{}-repo", project.slug),
                label: format!("{} repository resolves", project.name),
                source: format!("projects.{}.repo", project.slug),
                kind: CheckKind::ExternalLink {
                    url: repo.to_owned(),
                },
            });
        }

        if let Some(url) = project.url.as_deref().filter(|url| !is_placeholder(url)) {
            checks.push(Check {
                id: format!("project-{}-url", project.slug),
                label: format!("{} site resolves", project.name),
                source: format!("projects.{}.url", project.slug),
                kind: CheckKind::ExternalLink {
                    url: url.to_owned(),
                },
            });
        }
    }

    for path in ["/api/resume", "/api/schema", "/resume.pdf"] {
        checks.push(Check {
            id: format!("artifact{}", path.replace('/', "-")),
            label: format!("{path} is published"),
            source: "artifacts".to_owned(),
            kind: CheckKind::Artifact {
                path: path.to_owned(),
            },
        });
    }

    checks.truncate(MAX_CHECKS);
    checks
}

/// Run identifier: the plan plus the commit.
///
/// Changing content changes the plan and therefore the id, so a fresh publish
/// gets a fresh run rather than serving a stale one.
pub fn run_id(checks: &[Check], claimed_commit: &str) -> String {
    let mut material = String::from(claimed_commit);
    for check in checks {
        material.push('|');
        material.push_str(&check.id);
    }
    crate::fingerprint(material.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load;

    #[test]
    fn plan_is_deterministic() {
        let resume = load().unwrap();
        assert_eq!(plan(&resume, "abc1234"), plan(&resume, "abc1234"));
    }

    #[test]
    fn plan_skips_placeholder_urls() {
        let resume = load().unwrap();
        let checks = plan(&resume, "abc1234");

        // The content still has bracketed link placeholders, so none of them
        // should appear as external link checks.
        for check in &checks {
            if let CheckKind::ExternalLink { url } = &check.kind {
                assert!(!is_placeholder(url), "{url} is a placeholder");
            }
        }
    }

    #[test]
    fn plan_always_includes_commit_and_artifacts() {
        let resume = load().unwrap();
        let checks = plan(&resume, "abc1234");

        assert!(
            checks
                .iter()
                .any(|check| matches!(check.kind, CheckKind::LiveCommit { .. }))
        );
        assert_eq!(
            checks
                .iter()
                .filter(|check| matches!(check.kind, CheckKind::Artifact { .. }))
                .count(),
            3
        );
        assert!(checks.len() <= MAX_CHECKS);
    }

    #[test]
    fn repository_urls_are_plain_link_checks() {
        let resume = load().unwrap();
        let checks = plan(&resume, "abc1234");

        // Every check is now one of three kinds. A repository is a link like
        // any other, so no check reaches a third-party API whose rate limit
        // we neither control nor share fairly.
        for check in &checks {
            assert!(
                matches!(
                    check.kind,
                    CheckKind::ExternalLink { .. }
                        | CheckKind::Artifact { .. }
                        | CheckKind::LiveCommit { .. }
                ),
                "unexpected check kind: {:?}",
                check.kind
            );
        }
    }

    #[test]
    fn run_id_tracks_content_and_commit() {
        let resume = load().unwrap();
        let checks = plan(&resume, "abc1234");

        assert_eq!(run_id(&checks, "abc1234"), run_id(&checks, "abc1234"));
        assert_ne!(run_id(&checks, "abc1234"), run_id(&checks, "def5678"));

        let fewer = &checks[..checks.len() - 1];
        assert_ne!(run_id(&checks, "abc1234"), run_id(fewer, "abc1234"));
    }

    #[test]
    fn summary_separates_unreachable_from_wrong() {
        let mut run = CheckRun::queued("id", 3);
        run.status = RunStatus::Complete;
        run.results = vec![
            CheckResult {
                check_id: "a".into(),
                outcome: Outcome::Passed {
                    detail: "200".into(),
                },
                duration_ms: 10,
            },
            CheckResult {
                check_id: "b".into(),
                outcome: Outcome::Unreachable {
                    reason: "timed out".into(),
                },
                duration_ms: 5000,
            },
            CheckResult {
                check_id: "c".into(),
                outcome: Outcome::Mismatch {
                    expected: "abc1234".into(),
                    found: "def5678".into(),
                },
                duration_ms: 20,
            },
        ];

        assert_eq!(run.passed(), 1);
        assert_eq!(run.problems().count(), 1);
        assert_eq!(
            run.summary(),
            "1 of 3 checks passed, 1 unreachable, 1 mismatched"
        );
    }

    #[test]
    fn progress_never_reports_complete_while_running() {
        let mut run = CheckRun::queued("id", 2);
        run.status = RunStatus::Running;
        run.results = vec![CheckResult {
            check_id: "a".into(),
            outcome: Outcome::Passed {
                detail: "200".into(),
            },
            duration_ms: 1,
        }];
        assert_eq!(run.progress(), 50);

        run.results.push(CheckResult {
            check_id: "b".into(),
            outcome: Outcome::Passed {
                detail: "200".into(),
            },
            duration_ms: 1,
        });
        assert_eq!(run.progress(), 99, "still running, so not 100");

        run.status = RunStatus::Complete;
        assert_eq!(run.progress(), 100);
    }
}
