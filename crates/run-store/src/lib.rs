//! Storage for provenance check runs.
//!
//! A shared crate because the read and the write live in different processes:
//! the HTTP handler reads the projection to render a page, the task Lambda
//! writes it as the workflow progresses. Neither should depend on the other.
//!
//! # Why the item is not one JSON blob
//!
//! The obvious design, a partition key and a serialized `CheckRun`, is wrong
//! under a parallel `Map`. Every branch would read, modify, and write the whole
//! run, and concurrent branches would silently lose each other's results to
//! last-write-wins.
//!
//! So the item has real attributes and results are appended atomically with
//! `list_append`. Each branch's write touches only its own element, and no
//! branch needs to read the others.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::operation::put_item::PutItemError;
use aws_sdk_dynamodb::types::AttributeValue;
use resume_core::provenance::{CheckResult, CheckRun, RunStatus};

/// How long a run stays readable.
///
/// A run is disposable: losing one costs a visitor a re-click. An hour is long
/// enough to read a result and share the link, short enough that the table
/// never grows.
const TTL_SECONDS: u64 = 60 * 60;

const KEY: &str = "runId";
const STATUS: &str = "status";
const PLANNED: &str = "planned";
const RESULTS: &str = "results";
const DIAGNOSTIC: &str = "diagnostic";
const EXPIRES: &str = "expiresAt";

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("dynamodb request failed: {0}")]
    Dynamo(String),
    #[error("stored run was not readable: {0}")]
    Corrupt(#[from] serde_json::Error),
    #[error("stored run was missing required attributes")]
    Incomplete,
}

impl<E: std::fmt::Debug, R: std::fmt::Debug> From<SdkError<E, R>> for StoreError {
    fn from(error: SdkError<E, R>) -> Self {
        Self::Dynamo(format!("{error:?}"))
    }
}

/// In-process store. Correct in a single process and nowhere else.
#[derive(Clone, Default)]
pub struct MemoryRuns(Arc<Mutex<HashMap<String, CheckRun>>>);

impl MemoryRuns {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, CheckRun>> {
        // Poisoning means a holder panicked. The worst case is a lost run, so
        // recovering beats propagating.
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[derive(Clone)]
pub struct DynamoRuns {
    client: aws_sdk_dynamodb::Client,
    table: String,
}

/// An enum rather than a trait object: there will only ever be two, and
/// `async fn` in traits is not dyn-compatible.
#[derive(Clone)]
pub enum Runs {
    Memory(MemoryRuns),
    Dynamo(DynamoRuns),
}

impl Runs {
    pub fn memory() -> Self {
        Self::Memory(MemoryRuns::default())
    }

    pub async fn dynamo(table: impl Into<String>) -> Self {
        let config = aws_config::load_from_env().await;
        Self::Dynamo(DynamoRuns {
            client: aws_sdk_dynamodb::Client::new(&config),
            table: table.into(),
        })
    }

    /// Creates the run only if absent, returning whether this caller created it.
    ///
    /// The cache check that stops a second click starting a duplicate fan-out
    /// at third parties. On DynamoDB this is a conditional write, so it is
    /// genuinely atomic across instances rather than the pretend atomicity the
    /// in-memory variant offers.
    pub async fn claim(&self, run: &CheckRun) -> Result<bool, StoreError> {
        match self {
            Self::Memory(store) => {
                let mut guard = store.lock();
                if guard.contains_key(&run.id) {
                    return Ok(false);
                }
                guard.insert(run.id.clone(), run.clone());
                Ok(true)
            }
            Self::Dynamo(store) => {
                let result = store
                    .client
                    .put_item()
                    .table_name(&store.table)
                    .item(KEY, AttributeValue::S(run.id.clone()))
                    .item(STATUS, AttributeValue::S(run.status.as_str().to_owned()))
                    .item(PLANNED, AttributeValue::N(run.planned.to_string()))
                    .item(RESULTS, AttributeValue::L(Vec::new()))
                    .item(EXPIRES, AttributeValue::N(expires_at().to_string()))
                    .condition_expression("attribute_not_exists(#k)")
                    .expression_attribute_names("#k", KEY)
                    .send()
                    .await;

                match result {
                    Ok(_) => Ok(true),
                    // The expected outcome of a duplicate click, not an error.
                    Err(SdkError::ServiceError(service))
                        if matches!(
                            service.err(),
                            PutItemError::ConditionalCheckFailedException(_)
                        ) =>
                    {
                        tracing::info!(run = %run.id, "run already claimed, reusing");
                        Ok(false)
                    }
                    Err(error) => Err(StoreError::Dynamo(format!("{error:?}"))),
                }
            }
        }
    }

    /// Appends one result.
    ///
    /// `list_append` rather than read-modify-write, so parallel branches cannot
    /// overwrite each other. This is the whole reason the item has attributes
    /// instead of holding a serialized run.
    pub async fn append_result(&self, id: &str, result: &CheckResult) -> Result<(), StoreError> {
        match self {
            Self::Memory(store) => {
                if let Some(run) = store.lock().get_mut(id) {
                    run.results.push(result.clone());
                }
                Ok(())
            }
            Self::Dynamo(store) => {
                store
                    .client
                    .update_item()
                    .table_name(&store.table)
                    .key(KEY, AttributeValue::S(id.to_owned()))
                    .update_expression("SET #r = list_append(if_not_exists(#r, :empty), :new)")
                    .expression_attribute_names("#r", RESULTS)
                    .expression_attribute_values(":empty", AttributeValue::L(Vec::new()))
                    .expression_attribute_values(
                        ":new",
                        AttributeValue::L(vec![AttributeValue::S(serde_json::to_string(result)?)]),
                    )
                    .send()
                    .await?;
                Ok(())
            }
        }
    }

    pub async fn set_status(
        &self,
        id: &str,
        status: RunStatus,
        diagnostic: Option<&str>,
    ) -> Result<(), StoreError> {
        match self {
            Self::Memory(store) => {
                if let Some(run) = store.lock().get_mut(id) {
                    run.status = status;
                    run.diagnostic = diagnostic.map(str::to_owned);
                }
                Ok(())
            }
            Self::Dynamo(store) => {
                let mut request = store
                    .client
                    .update_item()
                    .table_name(&store.table)
                    .key(KEY, AttributeValue::S(id.to_owned()))
                    .expression_attribute_names("#s", STATUS)
                    .expression_attribute_values(
                        ":s",
                        AttributeValue::S(status.as_str().to_owned()),
                    );

                request = match diagnostic {
                    Some(text) => request
                        .update_expression("SET #s = :s, #d = :d")
                        .expression_attribute_names("#d", DIAGNOSTIC)
                        .expression_attribute_values(":d", AttributeValue::S(text.to_owned())),
                    None => request.update_expression("SET #s = :s"),
                };

                request.send().await?;
                Ok(())
            }
        }
    }

    pub async fn get(&self, id: &str) -> Result<Option<CheckRun>, StoreError> {
        match self {
            Self::Memory(store) => Ok(store.lock().get(id).cloned()),
            Self::Dynamo(store) => {
                let response = store
                    .client
                    .get_item()
                    .table_name(&store.table)
                    .key(KEY, AttributeValue::S(id.to_owned()))
                    // An eventually consistent read could show a visitor a
                    // half-finished run as though it were done.
                    .consistent_read(true)
                    .send()
                    .await?;

                let Some(item) = response.item else {
                    return Ok(None);
                };

                Ok(Some(from_item(id, &item)?))
            }
        }
    }
}

fn from_item(id: &str, item: &HashMap<String, AttributeValue>) -> Result<CheckRun, StoreError> {
    let status = item
        .get(STATUS)
        .and_then(|value| value.as_s().ok())
        .and_then(|value| RunStatus::parse(value))
        .ok_or(StoreError::Incomplete)?;

    let planned = item
        .get(PLANNED)
        .and_then(|value| value.as_n().ok())
        .and_then(|value| value.parse().ok())
        .ok_or(StoreError::Incomplete)?;

    let results = match item.get(RESULTS).and_then(|value| value.as_l().ok()) {
        Some(list) => list
            .iter()
            .filter_map(|value| value.as_s().ok())
            .map(|json| serde_json::from_str(json))
            .collect::<Result<Vec<CheckResult>, _>>()?,
        None => Vec::new(),
    };

    Ok(CheckRun {
        id: id.to_owned(),
        status,
        planned,
        results,
        diagnostic: item
            .get(DIAGNOSTIC)
            .and_then(|value| value.as_s().ok())
            .cloned(),
    })
}

fn expires_at() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    now + TTL_SECONDS
}

#[cfg(test)]
mod tests {
    use super::*;
    use resume_core::provenance::Outcome;

    fn result(id: &str) -> CheckResult {
        CheckResult {
            check_id: id.to_owned(),
            outcome: Outcome::Passed {
                detail: "HTTP 200".to_owned(),
            },
            duration_ms: 12,
        }
    }

    #[tokio::test]
    async fn a_run_is_claimed_once() {
        let runs = Runs::memory();
        let run = CheckRun::queued("abc", 3);

        assert!(runs.claim(&run).await.unwrap(), "first caller wins");
        assert!(!runs.claim(&run).await.unwrap(), "second caller reuses");
    }

    #[tokio::test]
    async fn results_accumulate_rather_than_replace() {
        let runs = Runs::memory();
        runs.claim(&CheckRun::queued("abc", 2)).await.unwrap();

        runs.append_result("abc", &result("one")).await.unwrap();
        runs.append_result("abc", &result("two")).await.unwrap();

        let stored = runs.get("abc").await.unwrap().expect("stored");
        assert_eq!(stored.results.len(), 2, "appends must not overwrite");
        assert_eq!(stored.planned, 2);
    }

    #[tokio::test]
    async fn status_and_diagnostic_are_updated_independently_of_results() {
        let runs = Runs::memory();
        runs.claim(&CheckRun::queued("abc", 1)).await.unwrap();
        runs.append_result("abc", &result("one")).await.unwrap();

        runs.set_status("abc", RunStatus::Failed, Some("origin unreachable"))
            .await
            .unwrap();

        let stored = runs.get("abc").await.unwrap().expect("stored");
        assert_eq!(stored.status, RunStatus::Failed);
        assert_eq!(stored.diagnostic.as_deref(), Some("origin unreachable"));
        assert_eq!(stored.results.len(), 1, "status change kept the results");
    }

    #[tokio::test]
    async fn an_unknown_run_is_absent_rather_than_an_error() {
        assert!(Runs::memory().get("nope").await.unwrap().is_none());
    }

    #[test]
    fn ttl_is_in_the_future() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(expires_at() > now);
    }
}
