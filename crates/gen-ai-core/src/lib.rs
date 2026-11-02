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
    Brief, CallStep, Check, Cursor, Diagram, DiagramGroup, DiagramNode, SCHEMA_VERSION, Scenario,
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

    /// A botocore operation name (PascalCase) to its Python method
    /// (snake_case): `StartIngestionJob` -> `start_ingestion_job`. An
    /// underscore goes before each interior uppercase letter, then the whole
    /// thing is lowercased.
    fn pascal_to_snake(op: &str) -> String {
        let mut out = String::new();
        for (i, ch) in op.chars().enumerate() {
            if i > 0 && ch.is_uppercase() {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        }
        out
    }

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
                // A runnable step must intercept something; a configuration
                // step legitimately intercepts nothing, because its lesson is
                // an artifact to read rather than a live call to run.
                if call.is_runnable() {
                    assert!(
                        !bench.responses.is_empty(),
                        "{} is runnable but intercepts nothing",
                        call.title
                    );
                }
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
                // botocore method names are the operation in snake_case:
                // Retrieve -> retrieve, StartIngestionJob -> start_ingestion_job.
                // A lowercase-only transform would miss the multi-word ones,
                // so insert an underscore before each interior capital.
                for response in &bench.responses {
                    let method = pascal_to_snake(&response.operation);
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

    /// The python snippets of a runnable step, concatenated in authored order
    /// the way a visitor assembles them into the open file. Non-python
    /// snippets (a shell sidecar, a CDK/TypeScript trigger shown for context)
    /// are not part of the runnable program and are excluded, the same way the
    /// visitor would not paste them into a `.py` file.
    #[cfg(test)]
    fn assembled_python(call: &CallStep) -> String {
        call.snippets
            .iter()
            .filter(|s| s.language == "python")
            .map(|s| s.source.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    #[test]
    fn a_runnable_steps_python_defines_before_it_uses() {
        // The NameError class: a snippet calls `agent.retrieve(...)` but no
        // snippet ever built `agent`. Individually every snippet is valid and
        // reachable, so the structural checks pass; only assembling and
        // reading them in order shows the client was never constructed. This
        // is a cheap static stand-in for that — not a Python parser, just the
        // one ordering fact that bit us: a client name is assigned before it
        // is called. The execution oracle in smoke-test.mjs is the real proof;
        // this fails fast, locally, without booting Pyodide.
        for scenario in catalog().unwrap() {
            for step in &scenario.steps {
                let Some(call) = step.as_call() else { continue };
                if !call.is_runnable() {
                    continue;
                }

                let program = assembled_python(call);

                // The client names this project uses. For each, if the program
                // calls `name.` anywhere, an assignment `name =` or `name=`
                // must appear at or before the first call site.
                for client in ["agent", "runtime", "agent_build"] {
                    let call_at = program.find(&format!("{client}."));
                    let Some(call_pos) = call_at else { continue };

                    let assign = program
                        .find(&format!("{client} ="))
                        .or_else(|| program.find(&format!("{client}=")));

                    match assign {
                        Some(assign_pos) => assert!(
                            assign_pos < call_pos,
                            "{}: '{}' is called before it is assigned in the assembled snippets",
                            call.title,
                            client
                        ),
                        None => panic!(
                            "{}: assembled snippets call '{}.' but never assign '{}' \
                             (the NameError this guard exists for)",
                            call.title, client, client
                        ),
                    }
                }

                // boto3 has to be imported for any of those clients to exist.
                if program.contains("boto3.client") {
                    assert!(
                        program.contains("import boto3"),
                        "{}: assembled snippets use boto3 but never import it",
                        call.title
                    );
                }
            }
        }
    }

    #[test]
    fn a_runnable_steps_open_file_is_python_and_so_are_its_runnable_snippets() {
        // The SyntaxError class: a CDK/TypeScript snippet tagged `python` gets
        // pasted into a `.py` file and Run throws a Python SyntaxError. The
        // open file of a runnable step is Python, so any snippet the visitor is
        // meant to assemble *into* it must be Python too. A snippet in another
        // language (a shell sidecar, a CDK trigger shown for context) is fine
        // as long as it is tagged honestly, because then it reads as context
        // rather than as something to paste and run.
        for scenario in catalog().unwrap() {
            for step in &scenario.steps {
                let Some(call) = step.as_call() else { continue };
                if !call.is_runnable() {
                    continue;
                }

                assert!(
                    call.workbench.open.ends_with(".py"),
                    "{}: a runnable step should open a .py file, opens {}",
                    call.title,
                    call.workbench.open
                );

                // A TypeScript snippet mistagged as python is the exact trap:
                // the `new` keyword and `const`/`=>` do not exist in Python, so
                // a snippet claiming to be python must not contain them.
                for snippet in &call.snippets {
                    if snippet.language != "python" {
                        continue;
                    }
                    for ts_only in ["new ", "const ", "=> {", ");\n};"] {
                        assert!(
                            !snippet.source.contains(ts_only),
                            "{}: a snippet tagged python contains TypeScript ('{}') — \
                             it will SyntaxError when assembled into {}",
                            call.title,
                            ts_only.trim(),
                            call.workbench.open
                        );
                    }
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
    fn every_config_check_can_be_satisfied_by_assembling_the_snippets() {
        // The answer-key integrity test, and the guard for the step-20 bug: a
        // check pattern is matched as a case-sensitive substring of the open
        // file, and the visitor fills that file by assembling the snippets. So
        // a `present: true` pattern that appears in no snippet can never pass
        // no matter how correctly the step is done — the check said "token
        // quota" lowercase while the snippet wrote "Tokens" capitalized, so
        // Check reported a failure on a correctly assembled artifact. And a
        // `present: false` pattern that IS in a snippet the step tells the
        // visitor to assemble fails the moment they follow the instructions.
        //
        // Only the snippet sources count: runChecks() reads the editor
        // textarea, not NOTES or labels, so a forbidden string living in the
        // prose (every absence lesson names the thing it forbids) is fine.
        for scenario in catalog().unwrap() {
            for step in &scenario.steps {
                let Some(call) = step.as_call() else { continue };

                let assembled = call
                    .snippets
                    .iter()
                    .map(|s| s.source.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");

                for check in &call.workbench.checks {
                    let found = assembled.contains(&check.pattern);
                    if check.present {
                        assert!(
                            found,
                            "{}: required check pattern {:?} is in no snippet, so \
                             Check can never pass on a correctly assembled artifact",
                            call.title, check.pattern
                        );
                    } else {
                        assert!(
                            !found,
                            "{}: forbidden check pattern {:?} is in a snippet the step \
                             tells the visitor to assemble, so Check fails when they do",
                            call.title, check.pattern
                        );
                    }
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
