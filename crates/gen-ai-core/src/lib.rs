//! The GenAI lab: scenario model, strict parsing, and markup.
//!
//! This crate has no `aws-sdk-*` and no `lambda_*` dependency, and that ban is
//! the point of it. The scenario logic and the renderer stay unit-testable
//! without AWS, and the markup functions stay callable from a publish task if
//! reads ever justify pre-rendering these pages.
//!
//! Nothing here calls a foundation model. A scenario is authored content
//! embedded at build time, which is what keeps a public route free of model
//! spend and of an abuse surface.

pub mod assets;
pub mod diagram;
pub mod render;
pub mod scenario;

pub use assets::Asset;
pub use diagram::Layout;
pub use render::{content_security_policy, landing, lesson, step_page};
pub use scenario::{
    Brief, CallStep, Cursor, Diagram, DiagramGroup, DiagramNode, SCHEMA_VERSION, Scenario,
    ScenarioError, Snippet, Step,
};

/// Every authored scenario, embedded at build time.
///
/// A slice of sources rather than a directory read: the handler must not touch
/// a file system at request time, and a missing document should fail the build
/// rather than a request.
const SOURCES: &[&str] = &[include_str!(
    "../../../content/gen-ai/policy-assistant.json"
)];

/// Parses every scenario, in catalog order.
///
/// Called once at cold start. A malformed document is a build-time bug that a
/// test catches, so the caller is entitled to treat a failure here as fatal.
pub fn catalog() -> Result<Vec<Scenario>, ScenarioError> {
    SOURCES
        .iter()
        .map(|source| Scenario::parse(source))
        .collect()
}

/// Finds a scenario by id.
pub fn find<'a>(catalog: &'a [Scenario], id: &str) -> Option<&'a Scenario> {
    catalog.iter().find(|scenario| scenario.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_embedded_scenario_parses() {
        let catalog = catalog().expect("embedded scenarios must be valid");

        assert!(!catalog.is_empty());
        for scenario in &catalog {
            assert_eq!(scenario.schema_version, SCHEMA_VERSION);
            assert!(!scenario.name.is_empty());
            assert!(!scenario.summary.is_empty());
        }
    }

    #[test]
    fn scenario_ids_are_unique() {
        let catalog = catalog().unwrap();

        let mut ids: Vec<&str> = catalog.iter().map(|s| s.id.as_str()).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), total, "duplicate scenario id");
    }

    #[test]
    fn every_scenario_states_what_breaks_it_and_what_done_means() {
        for scenario in catalog().unwrap() {
            let brief = &scenario.brief;

            for (field, value) in [
                ("whoIsAsking", &brief.who_is_asking),
                ("whatTheyNeedBack", &brief.what_they_need_back),
                ("systemsAlreadyRunning", &brief.systems_already_running),
                ("breaksWhen", &brief.breaks_when),
                ("doneMeans", &brief.done_means),
            ] {
                assert!(
                    !value.trim().is_empty(),
                    "{} has an empty {field}",
                    scenario.id
                );
            }

            // The two are not restatements of each other. A document that
            // pasted one into the other would pass every other check.
            assert_ne!(
                brief.breaks_when, brief.done_means,
                "{} states the same thing as its failure and its acceptance test",
                scenario.id
            );
        }
    }

    #[test]
    fn a_scenario_opens_on_the_overview_and_only_there() {
        for scenario in catalog().unwrap() {
            assert!(
                !scenario.steps[0].has_console(),
                "{} opens with a console",
                scenario.id
            );

            // Every later step is a call. An overview in the middle would mean
            // the console appears, disappears, and comes back.
            for step in &scenario.steps[1..] {
                assert!(
                    step.has_console(),
                    "{} has a second overview at '{}'",
                    scenario.id,
                    step.title()
                );
            }
        }
    }

    #[test]
    fn every_call_step_seeds_a_workbench_it_can_open() {
        for scenario in catalog().unwrap() {
            for step in &scenario.steps {
                let Some(call) = step.as_call() else { continue };
                let bench = &call.workbench;

                assert!(!bench.files.is_empty(), "{} seeds nothing", call.title);
                assert!(
                    bench.files.iter().any(|f| f.path == bench.open),
                    "{} opens {} but does not seed it",
                    call.title,
                    bench.open
                );
                assert!(
                    !bench.responses.is_empty(),
                    "{} intercepts nothing, so nothing it runs can answer",
                    call.title
                );
            }
        }
    }

    #[test]
    fn seeded_code_only_calls_operations_the_step_answers() {
        for scenario in catalog().unwrap() {
            for step in &scenario.steps {
                let Some(call) = step.as_call() else { continue };
                let bench = &call.workbench;

                // An intercepted response that nothing teaches the visitor to
                // call is content nobody will reach. The snippets are what the
                // visitor assembles and runs, so every intercepted operation
                // should appear in at least one snippet.
                //
                // botocore method names are the operation lowercased:
                // Retrieve becomes retrieve, Converse becomes converse.
                for response in &bench.responses {
                    let method = response.operation.to_lowercase();
                    let taught = call
                        .snippets
                        .iter()
                        .any(|snippet| snippet.source.contains(&format!(".{method}(")));

                    assert!(
                        taught,
                        "{} intercepts {}.{} but no snippet calls it",
                        call.title, response.service, response.operation
                    );
                }
            }
        }
    }

    #[test]
    fn snippets_are_not_obviously_broken() {
        for scenario in catalog().unwrap() {
            for step in &scenario.steps {
                let Some(call) = step.as_call() else { continue };

                // The snippets are the code the visitor types. Tabs are the
                // one thing that silently breaks Python assembled from them,
                // because a mix of tabs and the spaces they will type is an
                // IndentationError they did not cause. Not a parser; just the
                // one trap worth catching at author time.
                for snippet in &call.snippets {
                    assert!(
                        !snippet.source.contains('\t'),
                        "{} has a snippet with a tab",
                        call.title
                    );
                    assert!(
                        !snippet.label.trim().is_empty(),
                        "{} has an unlabelled snippet, so the reader cannot tell what it is",
                        call.title
                    );
                }
            }
        }
    }

    #[test]
    fn the_seed_is_a_scaffold_not_the_answer() {
        for scenario in catalog().unwrap() {
            for step in &scenario.steps {
                let Some(call) = step.as_call() else { continue };

                let open = call
                    .workbench
                    .files
                    .iter()
                    .find(|f| f.path == call.workbench.open)
                    .expect("the open file is seeded");

                // The point of the model: the editor does not start with the
                // finished call. If the seed already contained every operation
                // the step teaches, there would be nothing left to write.
                let all_present = call.workbench.responses.iter().all(|r| {
                    let method = r.operation.to_lowercase();
                    open.body.contains(&format!(".{method}("))
                });

                assert!(
                    !all_present || call.workbench.responses.is_empty(),
                    "{} seeds the finished answer, leaving nothing to assemble",
                    call.title
                );
            }
        }
    }

    #[test]
    fn every_reason_can_actually_fire() {
        for scenario in catalog().unwrap() {
            for step in &scenario.steps {
                let Some(call) = step.as_call() else { continue };

                for reason in &call.workbench.reasons {
                    assert!(
                        !reason.on_exception.trim().is_empty(),
                        "{} has a reason keyed on no exception",
                        call.title
                    );
                    assert!(
                        !reason.prose.trim().is_empty(),
                        "{} has an empty reason for {}",
                        call.title,
                        reason.on_exception
                    );
                    // A reason is the sentence an exception cannot supply. One
                    // that only restates the class name is not that.
                    assert!(
                        reason.prose.len() > reason.on_exception.len(),
                        "{} adds nothing beyond the exception name",
                        reason.on_exception
                    );
                }
            }
        }
    }

    #[test]
    fn find_matches_by_id_and_misses_cleanly() {
        let catalog = catalog().unwrap();
        let first = catalog[0].id.clone();

        assert!(find(&catalog, &first).is_some());
        assert!(find(&catalog, "not-a-scenario").is_none());
    }
}
