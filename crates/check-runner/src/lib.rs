//! Executes a single provenance check.
//!
//! A separate crate because two callers need it: the HTTP handler running the
//! feature locally, and the Step Functions task Lambda running it deployed.
//! Neither should depend on the other.
//!
//! `resume-core` deliberately has no HTTP dependency, which is why the
//! planning and interpretation live there and the network calls live here.

use std::time::{Duration, Instant};

use resume_core::provenance::{Check, CheckKind, CheckResult, Outcome};
use serde::Deserialize;

/// Per-request timeout. Generous enough for a slow corporate site, short
/// enough that a blackholed host does not hold a branch open.
const TIMEOUT: Duration = Duration::from_secs(8);

/// Attempts per check, including the first.
///
/// Two. The point is to absorb a blip, not to hammer a struggling host, and a
/// visitor is waiting on the result.
const ATTEMPTS: u32 = 2;

/// Pause before the second attempt. Long enough to clear a transient fault,
/// short enough that nobody notices.
const RETRY_DELAY: Duration = Duration::from_millis(400);

/// Sent on every outbound request. Being identifiable is the polite minimum
/// when fanning out at someone else's infrastructure.
const USER_AGENT: &str = "rader-resume-provenance/1.0 (+https://rader-resume.com)";

pub struct Checker {
    client: reqwest::Client,
    /// Origin for artifact and live-commit checks, with no trailing slash.
    origin: String,
}

impl Checker {
    pub fn new(origin: impl Into<String>) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .user_agent(USER_AGENT)
            // A link that redirects is still a working link, but a redirect
            // chain is not worth following forever.
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()?;

        Ok(Self {
            client,
            origin: origin.into().trim_end_matches('/').to_owned(),
        })
    }

    pub async fn run(&self, check: &Check) -> CheckResult {
        let started = Instant::now();

        let outcome = match &check.kind {
            CheckKind::ExternalLink { url } => self.external_link(url).await,
            CheckKind::Artifact { path } => self.artifact(path).await,
            CheckKind::LiveCommit { claimed } => self.live_commit(claimed).await,
        };

        CheckResult {
            check_id: check.id.clone(),
            outcome,
            duration_ms: started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32,
        }
    }

    /// Status interpretation is the crux of this feature.
    ///
    /// A 404 means the link is genuinely dead, which is a real problem worth
    /// reporting. A 403 or 429 means a bot filter or a rate limit, which says
    /// nothing about the link and everything about who is asking. A 5xx is
    /// their outage. Conflating these would make the report dishonest.
    fn classify(status: reqwest::StatusCode) -> Outcome {
        if status.is_success() || status.is_redirection() {
            return Outcome::Passed {
                detail: format!("HTTP {}", status.as_u16()),
            };
        }

        match status.as_u16() {
            404 | 410 => Outcome::Mismatch {
                expected: "a working link".to_owned(),
                found: format!("HTTP {}", status.as_u16()),
            },
            401 | 403 | 429 => Outcome::Unreachable {
                reason: format!("blocked or rate limited (HTTP {})", status.as_u16()),
            },
            other => Outcome::Unreachable {
                reason: format!("HTTP {other}"),
            },
        }
    }

    /// Whether a second attempt could plausibly answer differently.
    ///
    /// Deliberately narrow, and the exclusions matter more than the
    /// inclusions:
    ///
    /// - A 404 or 410 is a definite answer. Asking twice does not make a dead
    ///   link less dead.
    /// - A 401 or 403 is an auth wall or a bot filter. A retry from the same
    ///   address cannot pass it, so retrying is pointless and impolite.
    /// - A 429 is an explicit instruction to slow down. Retrying disobeys the
    ///   one thing the server asked for.
    /// - A timeout is excluded, which is the non-obvious one. A host that
    ///   spent the whole eight second budget not answering is not having a
    ///   blip, and retrying it is what would push a run past the task's
    ///   twenty second ceiling: two full timeouts plus the delay is 16.4
    ///   seconds before the result is even written.
    ///
    /// That leaves refused or reset connections and 5xx responses: faults that
    /// fail fast and may genuinely clear a moment later. Those are the ones
    /// this retry exists for, and retrying them costs almost nothing.
    /// Split from the response so it is testable against a bare status, the
    /// same shape as [`Checker::classify`].
    fn status_is_worth_retrying(status: reqwest::StatusCode) -> bool {
        status.is_server_error()
    }

    fn is_worth_retrying(result: &Result<reqwest::Response, reqwest::Error>) -> bool {
        match result {
            Ok(response) => Self::status_is_worth_retrying(response.status()),
            Err(error) => error.is_connect(),
        }
    }

    /// A GET that absorbs a fast, transient failure before reporting one.
    ///
    /// Without this, a reset connection becomes `Unreachable`, which is an
    /// honest answer to the wrong question: the target is reachable, the
    /// single attempt just happened to land badly. The Step Functions retry
    /// cannot help here, because a caught transport error means the task
    /// *succeeded* and returned a result, so nothing fires.
    async fn get(&self, url: &str) -> Result<reqwest::Response, reqwest::Error> {
        let mut remaining = ATTEMPTS;

        loop {
            let result = self.client.get(url).send().await;
            remaining -= 1;

            if remaining == 0 || !Self::is_worth_retrying(&result) {
                return result;
            }

            tokio::time::sleep(RETRY_DELAY).await;
        }
    }

    fn transport_failure(error: &reqwest::Error) -> Outcome {
        let reason = if error.is_timeout() {
            "timed out".to_owned()
        } else if error.is_connect() {
            "could not connect".to_owned()
        } else {
            "request failed".to_owned()
        };

        Outcome::Unreachable { reason }
    }

    async fn external_link(&self, url: &str) -> Outcome {
        match self.get(url).await {
            Ok(response) => Self::classify(response.status()),
            Err(error) => Self::transport_failure(&error),
        }
    }

    /// Artifacts are served by this site, so a failure is our problem and a
    /// genuine mismatch rather than something to excuse as unreachable.
    async fn artifact(&self, path: &str) -> Outcome {
        let url = format!("{}{path}", self.origin);

        match self.get(&url).await {
            Ok(response) if response.status().is_success() => Outcome::Passed {
                detail: format!("HTTP {}", response.status().as_u16()),
            },
            Ok(response) => Outcome::Mismatch {
                expected: "a published artifact".to_owned(),
                found: format!("HTTP {}", response.status().as_u16()),
            },
            Err(error) => Self::transport_failure(&error),
        }
    }

    async fn live_commit(&self, claimed: &str) -> Outcome {
        #[derive(Deserialize)]
        struct Build {
            commit: String,
        }

        let url = format!("{}/api/build", self.origin);

        let response = match self.get(&url).await {
            Ok(response) => response,
            Err(error) => return Self::transport_failure(&error),
        };

        match response.json::<Build>().await {
            Ok(build) if build.commit == claimed => Outcome::Passed {
                detail: format!("serving {claimed}"),
            },
            Ok(build) => Outcome::Mismatch {
                expected: claimed.to_owned(),
                found: build.commit,
            },
            Err(_) => Outcome::Unreachable {
                reason: "build endpoint did not answer".to_owned(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    #[test]
    fn dead_links_are_mismatches_and_blocks_are_not() {
        assert!(matches!(
            Checker::classify(StatusCode::NOT_FOUND),
            Outcome::Mismatch { .. }
        ));
        assert!(matches!(
            Checker::classify(StatusCode::GONE),
            Outcome::Mismatch { .. }
        ));

        for status in [
            StatusCode::FORBIDDEN,
            StatusCode::UNAUTHORIZED,
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::BAD_GATEWAY,
        ] {
            assert!(
                matches!(Checker::classify(status), Outcome::Unreachable { .. }),
                "{status} should not be reported as a failed claim"
            );
        }
    }

    #[test]
    fn success_and_redirects_pass() {
        for status in [
            StatusCode::OK,
            StatusCode::MOVED_PERMANENTLY,
            StatusCode::FOUND,
        ] {
            assert!(matches!(Checker::classify(status), Outcome::Passed { .. }));
        }
    }

    #[test]
    fn only_fast_transient_faults_are_retried() {
        // Server faults may clear.
        for status in [
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::BAD_GATEWAY,
            StatusCode::SERVICE_UNAVAILABLE,
        ] {
            assert!(
                Checker::status_is_worth_retrying(status),
                "{status} may clear and is worth a second attempt"
            );
        }

        // Definite answers and refusals are not retried. Asking a 404 twice
        // does not make it less dead, a 403 bot filter will refuse again, and
        // a 429 asked us to stop.
        for status in [
            StatusCode::NOT_FOUND,
            StatusCode::GONE,
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::OK,
        ] {
            assert!(
                !Checker::status_is_worth_retrying(status),
                "{status} should not be retried"
            );
        }
    }

    #[test]
    fn the_retry_budget_fits_inside_the_task_timeout() {
        // Timeouts are excluded from retry precisely so this holds. Two full
        // timeouts plus the delay would be 16.4s, leaving almost nothing of
        // the task's 20s for a cold start and the result write.
        let worst_case = TIMEOUT + RETRY_DELAY;
        assert!(
            worst_case < Duration::from_secs(20),
            "a single check can outlast the task Lambda"
        );
    }

    #[test]
    fn origin_trailing_slash_is_normalized() {
        let checker = Checker::new("https://example.com/").unwrap();
        assert_eq!(checker.origin, "https://example.com");
    }
}
