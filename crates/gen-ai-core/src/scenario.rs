//! The scenario model.
//!
//! A scenario is an authored document, embedded at build time. Nothing here
//! reads a file system at request time and nothing calls a model: a scenario
//! is content, which is what keeps this route free of spend and of an abuse
//! surface.
//!
//! Parsing is strict. An unknown field in an authored document is a typo, the
//! same rule the resume's content document follows.

use serde::{Deserialize, Serialize};

/// Schema version of an authored scenario.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ScenarioError {
    #[error("scenario document is malformed: {0}")]
    Malformed(#[from] serde_json::Error),
    #[error("scenario {id} is schema version {found}, expected {SCHEMA_VERSION}")]
    WrongVersion { id: String, found: u32 },
    #[error("scenario {id} has no steps")]
    NoSteps { id: String },
    #[error("scenario {id} does not open on an overview")]
    NoOverview { id: String },
    #[error("scenario {id} has a duplicate step title: {title}")]
    DuplicateTitle { id: String, title: String },
    #[error("scenario {id} diagram edge names {target}, which is not a node")]
    DanglingEdge { id: String, target: String },
    #[error("scenario {id} step '{title}' seeds no files, so its workbench is empty")]
    NoSeedFiles { id: String, title: String },
    #[error("scenario {id} step '{title}' opens {path}, which it does not seed")]
    OpensMissingFile {
        id: String,
        title: String,
        path: String,
    },
    #[error(
        "scenario {id} step '{title}' is a configuration step with no checks, so nothing verifies the assembled artifact"
    )]
    ConfigStepHasNoChecks { id: String, title: String },
    #[error("scenario {id} step '{title}' shows no snippets, so there is nothing to assemble")]
    NoSnippets { id: String, title: String },
    #[error(
        "scenario {id} step '{title}' has a snippet in an unknown language '{language}'; the renderer only labels python, shell, and typescript"
    )]
    UnknownSnippetLanguage {
        id: String,
        title: String,
        language: String,
    },
    #[error(
        "scenario {id} step '{title}' is runnable and opens the python file {open}, but none of its snippets are python, so there is nothing to assemble and run"
    )]
    RunnableStepHasNoPython {
        id: String,
        title: String,
        open: String,
    },
    #[error(
        "scenario {id} step '{title}' is runnable, so it shows the terminal, but has no shell snippet telling the visitor what to type into it — add a shell snippet with the aws command this step teaches"
    )]
    RunnableStepHasNoShell { id: String, title: String },
}

/// The languages the renderer knows how to label. A snippet in anything else
/// is an authoring typo (`py`, `ts`) that would render an unlabelled or
/// mislabelled chip, so parsing rejects it.
const KNOWN_SNIPPET_LANGUAGES: [&str; 3] = ["python", "shell", "typescript"];

/// The job a scenario puts you inside.
///
/// Five named fields rather than a paragraph. A sentence that only names the
/// corpus is not a brief, and five fields make that unrepresentable instead of
/// merely discouraged.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Brief {
    pub who_is_asking: String,
    pub what_they_need_back: String,
    pub systems_already_running: String,
    /// The roadmap's `Breaks when`: the failure that invalidates the
    /// architecture, not one that closes as a bug ticket.
    ///
    /// Separate from `done_means` on purpose. This is what the step decisions
    /// argue against, because each step's losing product loses by walking
    /// toward it.
    pub breaks_when: String,
    /// The roadmap's `Done means`: the acceptance test.
    pub done_means: String,
}

/// One teaching snippet in the walkthrough.
///
/// The step does not ship the finished file. It shows the pieces, each with a
/// label saying what the piece does, and the visitor assembles them in the
/// editor. Reading the snippet is the prompt; writing it is the practice.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snippet {
    /// What this piece is, e.g. "Retrieve the policy chunks".
    pub label: String,
    /// `python` or `shell`. Drives the label, not execution.
    pub language: String,
    pub source: String,
}

/// A file seeded into the workbench when a step mounts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SeedFile {
    pub path: String,
    pub body: String,
}

/// What the intercepted transport returns for one operation.
///
/// Keyed by the botocore service and operation, so the genuine client
/// validates and serializes the call and only the network hop is replaced.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Intercepted {
    /// botocore service id, such as `bedrock-agent-runtime`.
    pub service: String,
    /// botocore operation name, such as `Retrieve`.
    pub operation: String,
    /// Returned as the parsed response body.
    pub body: serde_json::Value,
}

/// Prose the lesson adds beside a genuine exception.
///
/// Keyed on the exception the real library raised, which is the inversion
/// that matters: botocore has already decided precisely what was wrong, so
/// the lesson supplies only the sentence an exception cannot, which is which
/// product the step uses and why the other one loses.
///
/// A reason never replaces the error. An error with no reason beside it still
/// teaches, because it is the real error.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reason {
    /// Exception class name, such as `ParamValidationError`.
    pub on_exception: String,
    /// Optional narrowing against the message, for when one class covers
    /// several distinct mistakes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when_message_contains: Option<String>,
    pub prose: String,
}

/// The environment a call step mounts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Workbench {
    /// Seeded into the in-memory filesystem. A scaffold, not the answer: the
    /// visitor assembles the snippets from the walkthrough into it. A file may
    /// carry a stub or reference notes, but the finished call is what the
    /// visitor writes, not what the step ships.
    pub files: Vec<SeedFile>,
    /// Which seeded file the single editor opens on when the step mounts. The
    /// editor tracks every seeded file and switches between them from the file
    /// tree, so this names the starting file rather than the only one. The
    /// action button follows the open file's extension: a `.py` file runs
    /// against the intercept, a `.ts` file runs the step's `checks`.
    pub open: String,
    /// What the intercepted transport returns, keyed by operation.
    ///
    /// Empty for a configuration step — one whose lesson is a CDK or config
    /// artifact to read and assemble, not a live AWS call. Not every teaching
    /// step is a runnable call: chunking strategy, an IAM policy, a
    /// PrivateLink endpoint are decisions you configure, and forcing a fake
    /// response onto them would be dishonest. A step with responses runs
    /// against the intercept; a step without is config. The `is_runnable`
    /// helper is the one place that distinction is read, and it is also what
    /// decides whether the always-on terminal appears beside the editor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub responses: Vec<Intercepted>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<Reason>,
    /// Structural checks for a configuration step, run in the browser when the
    /// visitor presses Run on a step that has no live call.
    ///
    /// A config step seeds a CDK or config artifact to assemble, and the
    /// sandbox cannot deploy it to tell you it is right. These checks are the
    /// answer key: each asserts the assembled text must, or must not, contain
    /// a pattern, with a message explaining the decision. The absence checks
    /// carry half the lessons — no fine-tune job, no wildcard IAM — which a
    /// 'does it contain' test alone cannot express.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<Check>,
}

/// One structural check over the assembled artifact.
///
/// `pattern` is matched as a plain substring (case-sensitive), which is enough
/// for assembled snippets: the visitor is composing provided pieces, so the
/// check confirms the right pieces are present in the right shape rather than
/// parsing a language. `present` says whether the pattern must appear
/// (`true`) or must be absent (`false`). `message` is shown pass or fail, so
/// the reasoning is visible either way.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Check {
    pub pattern: String,
    /// Must the pattern be present (`true`) or absent (`false`)?
    pub present: bool,
    /// The decision this check verifies, shown whether it passes or fails.
    pub message: String,
}

/// A call step: one decision, with its editor, its terminal, and its
/// environment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallStep {
    pub title: String,
    /// What you are deciding, in this job.
    pub decision: String,
    /// Why this product, and what the other choice gets wrong.
    pub why: String,
    /// The pieces of the call, each labelled. The visitor reads these and
    /// assembles them in the editor; the step does not ship the finished file.
    pub snippets: Vec<Snippet>,
    /// CDK for the same step, in the dialect `infra/` uses.
    pub infrastructure: String,
    pub workbench: Workbench,
}

impl CallStep {
    /// Whether this step runs a live AWS call against the intercept.
    ///
    /// A runnable step has intercepted responses and teaches by running the
    /// assembled call. A configuration step has none: its lesson is the CDK or
    /// config artifact in the editor, read and assembled, with nothing to send
    /// to a transport. The renderer uses this to label the run affordance, and
    /// the lib tests use it to apply response-reachability only where it means
    /// something.
    pub fn is_runnable(&self) -> bool {
        !self.workbench.responses.is_empty()
    }
}

/// The architecture drawing on step 1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Diagram {
    pub nodes: Vec<DiagramNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<DiagramGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagramNode {
    pub id: String,
    pub label: String,
    /// AWS Architecture Icon key.
    ///
    /// `None` renders the name alone, which is both this phase's behaviour and
    /// the permanent fallback if the icon set cannot be shipped. Carrying the
    /// field from the start means adding icons is not a content migration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Group id, for the dashed Region or VPC outline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flows_to: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagramGroup {
    pub id: String,
    pub label: String,
}

/// One step of a scenario.
///
/// An enum rather than a struct with optional panes, because the difference is
/// a capability: the overview has no console and cannot take a command, and a
/// call can. That belongs in the type instead of in a runtime check repeated
/// across handlers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Step {
    /// Step 1. The architecture, full width, no console.
    Overview { title: String, diagram: Diagram },
    /// Step 2 onward. One call.
    Call(CallStep),
}

impl Step {
    pub fn title(&self) -> &str {
        match self {
            Self::Overview { title, .. } => title,
            Self::Call(call) => &call.title,
        }
    }

    /// The one place the variant is checked.
    ///
    /// A step arrives as an integer on the wire, so which variant it resolves
    /// to cannot be known at compile time and one runtime check is
    /// unavoidable. Keeping it here means everything downstream takes
    /// `&CallStep` and is statically safe: phase 2's command checker cannot be
    /// handed the overview, because the signature forbids it.
    pub fn as_call(&self) -> Option<&CallStep> {
        match self {
            Self::Overview { .. } => None,
            Self::Call(call) => Some(call),
        }
    }

    /// Whether this step shows the console column.
    ///
    /// Derived from the variant rather than stored, so a document cannot claim
    /// an overview with a console.
    pub fn has_console(&self) -> bool {
        self.as_call().is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scenario {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    /// One line for the landing list.
    pub summary: String,
    pub brief: Brief,
    pub steps: Vec<Step>,
}

impl Scenario {
    /// Parses and validates an authored document.
    ///
    /// Every check here is a build-time failure in practice, because documents
    /// are embedded. A scenario that opens on a call step, has a dangling
    /// diagram edge, or repeats a title is a mistake worth catching before it
    /// is a page.
    pub fn parse(source: &str) -> Result<Self, ScenarioError> {
        let scenario: Self = serde_json::from_str(source)?;

        if scenario.schema_version != SCHEMA_VERSION {
            return Err(ScenarioError::WrongVersion {
                id: scenario.id,
                found: scenario.schema_version,
            });
        }

        let Some(first) = scenario.steps.first() else {
            return Err(ScenarioError::NoSteps { id: scenario.id });
        };

        let Step::Overview { diagram, .. } = first else {
            return Err(ScenarioError::NoOverview { id: scenario.id });
        };

        let ids: Vec<&str> = diagram.nodes.iter().map(|node| node.id.as_str()).collect();
        for node in &diagram.nodes {
            for target in &node.flows_to {
                if !ids.contains(&target.as_str()) {
                    return Err(ScenarioError::DanglingEdge {
                        id: scenario.id.clone(),
                        target: target.clone(),
                    });
                }
            }
        }

        let mut seen: Vec<&str> = Vec::new();
        for step in &scenario.steps {
            if seen.contains(&step.title()) {
                return Err(ScenarioError::DuplicateTitle {
                    id: scenario.id.clone(),
                    title: step.title().to_owned(),
                });
            }
            seen.push(step.title());
        }

        // A workbench with nothing in it is a promise the step does not keep,
        // so these are build failures rather than empty panes at runtime.
        for step in &scenario.steps {
            let Some(call) = step.as_call() else { continue };
            let bench = &call.workbench;

            if bench.files.is_empty() {
                return Err(ScenarioError::NoSeedFiles {
                    id: scenario.id.clone(),
                    title: call.title.clone(),
                });
            }

            if !bench.files.iter().any(|file| file.path == bench.open) {
                return Err(ScenarioError::OpensMissingFile {
                    id: scenario.id.clone(),
                    title: call.title.clone(),
                    path: bench.open.clone(),
                });
            }

            // A configuration step legitimately has no responses, but it must
            // give feedback some other way: it carries checks that verify the
            // assembled artifact. A config step with neither responses nor
            // checks is the worksheet-with-no-answer-key the design forbids —
            // the visitor assembles something and learns nothing about whether
            // it is right. A runnable step gets its feedback from running, so
            // it needs no checks.
            if call.workbench.responses.is_empty() && call.workbench.checks.is_empty() {
                return Err(ScenarioError::ConfigStepHasNoChecks {
                    id: scenario.id.clone(),
                    title: call.title.clone(),
                });
            }

            if call.snippets.is_empty() {
                return Err(ScenarioError::NoSnippets {
                    id: scenario.id.clone(),
                    title: call.title.clone(),
                });
            }

            // Every snippet language must be one the renderer can label. A
            // typo like `py` or `ts` would render a wrong or missing chip, and
            // the chip is now how a visitor tells an assemble-this snippet from
            // a reference one.
            for snippet in &call.snippets {
                if !KNOWN_SNIPPET_LANGUAGES.contains(&snippet.language.as_str()) {
                    return Err(ScenarioError::UnknownSnippetLanguage {
                        id: scenario.id.clone(),
                        title: call.title.clone(),
                        language: snippet.language.clone(),
                    });
                }
            }

            // A runnable step opens a python file and the visitor assembles the
            // python snippets into it. If none of its snippets are python there
            // is nothing valid to run — the step-9 failure in the limit, where
            // the only runnable snippet was crowded out by a CDK one. A config
            // step opens a stack.ts and assembles typescript, so this applies
            // only to runnable steps.
            if call.is_runnable()
                && bench.open.ends_with(".py")
                && !call.snippets.iter().any(|s| s.language == "python")
            {
                return Err(ScenarioError::RunnableStepHasNoPython {
                    id: scenario.id.clone(),
                    title: call.title.clone(),
                    open: bench.open.clone(),
                });
            }

            // A runnable step shows the always-on terminal, so it must teach a
            // command to type into it: at least one shell snippet. Without one
            // the terminal is a tool with no lesson — the gap the audit found
            // on the Retrieve/RetrieveAndGenerate steps. The shell snippet is
            // the CLI exercise, and its operation is answered by an authored
            // response so the command genuinely runs.
            if call.is_runnable() && !call.snippets.iter().any(|s| s.language == "shell") {
                return Err(ScenarioError::RunnableStepHasNoShell {
                    id: scenario.id.clone(),
                    title: call.title.clone(),
                });
            }
        }

        Ok(scenario)
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Always false. A scenario with no steps fails to parse, so this exists
    /// only to satisfy the lint that pairs with `len`.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

/// A position inside a scenario.
///
/// Not a typestate. Position gates no capability and has no illegal
/// transition, so encoding it as one would be ceremony. What it does own is
/// clamping, in one tested place, because the index arrives in a POST body and
/// is therefore untrusted input.
#[derive(Debug, Clone, Copy)]
pub struct Cursor<'a> {
    scenario: &'a Scenario,
    /// In range by construction.
    index: usize,
}

impl<'a> Cursor<'a> {
    /// Clamps rather than failing.
    ///
    /// A visitor cannot reach a step that does not exist, and a hand-edited
    /// form value lands on the nearest real step instead of a 400 that teaches
    /// nobody anything.
    pub fn at(scenario: &'a Scenario, requested: usize) -> Self {
        let last = scenario.len().saturating_sub(1);
        Self {
            scenario,
            index: requested.min(last),
        }
    }

    /// `next` and `previous` consume `self` so a handler cannot hold two
    /// positions at once and render the wrong one. Both saturate at the ends.
    pub fn next(self) -> Self {
        Self {
            index: (self.index + 1).min(self.scenario.len().saturating_sub(1)),
            ..self
        }
    }

    pub fn previous(self) -> Self {
        Self {
            index: self.index.saturating_sub(1),
            ..self
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    /// One-based, for display. A step index shown to a visitor starts at 1.
    pub fn number(&self) -> usize {
        self.index + 1
    }

    pub fn step(&self) -> &'a Step {
        &self.scenario.steps[self.index]
    }

    pub fn scenario(&self) -> &'a Scenario {
        self.scenario
    }

    pub fn is_first(&self) -> bool {
        self.index == 0
    }

    pub fn is_last(&self) -> bool {
        self.index + 1 >= self.scenario.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal two-step scenario (overview + one call) whose single call
    /// step's snippets and workbench are supplied as values. Built with
    /// `json!` rather than string splicing so the test stays free of
    /// raw-string escaping entirely. Enough to exercise the parse guards
    /// without carrying a whole authored document.
    fn scenario_with(snippets: serde_json::Value, workbench: serde_json::Value) -> String {
        serde_json::json!({
            "schemaVersion": 1,
            "id": "test",
            "name": "Test",
            "summary": "A test scenario.",
            "brief": {
                "whoIsAsking": "a",
                "whatTheyNeedBack": "b",
                "systemsAlreadyRunning": "c",
                "breaksWhen": "d",
                "doneMeans": "e"
            },
            "steps": [
                {
                    "kind": "overview",
                    "title": "Overview",
                    "diagram": { "nodes": [{ "id": "n", "label": "N" }] }
                },
                {
                    "kind": "call",
                    "title": "A call",
                    "decision": "d",
                    "why": "w",
                    "snippets": snippets,
                    "infrastructure": "",
                    "workbench": workbench
                }
            ]
        })
        .to_string()
    }

    #[test]
    fn a_step_with_both_python_and_typescript_files_parses_and_is_runnable() {
        // The worked case the single editor is built for: a step that seeds
        // both a .py file and a .ts file, carries responses (so it runs) and
        // checks (so the .ts file can be graded). The single editor tracks
        // both files; the action button follows whichever is open. Nothing
        // here names a second editor or a cli block — those concepts are gone
        // — so the step is well-formed on the strength of its files alone.
        let workbench = serde_json::json!({
            "open": "answer.py",
            "files": [
                { "path": "answer.py", "body": "# here\n" },
                { "path": "stack.ts", "body": "// here\n" }
            ],
            "responses": [{ "service": "s", "operation": "Op", "body": {} }],
            "checks": [{ "pattern": "Bucket", "present": true, "message": "a bucket" }]
        });
        // Python to assemble into answer.py, typescript to assemble into
        // stack.ts. The check pattern lives in the typescript snippet so the
        // answer-key integrity test is satisfiable.
        // Python to assemble into answer.py, typescript to assemble into
        // stack.ts, and a shell snippet for the always-on terminal a runnable
        // step shows — the guard requires all three where they apply.
        let snippets = serde_json::json!([
            { "label": "call", "language": "python", "source": "import boto3\nboto3.client(\"s\").op()" },
            { "label": "stack", "language": "typescript", "source": "const b = new Bucket();" },
            { "label": "verify", "language": "shell", "source": "aws s op" }
        ]);
        let source = scenario_with(snippets, workbench);
        let scenario =
            Scenario::parse(&source).expect("a step seeding both .py and .ts files parses");
        let call = scenario.steps[1].as_call().unwrap();

        assert!(call.is_runnable(), "a step with responses is runnable");
        // The editor tracks every seeded file; both are present to switch
        // between.
        let paths: Vec<&str> = call
            .workbench
            .files
            .iter()
            .map(|f| f.path.as_str())
            .collect();
        assert!(paths.contains(&"answer.py"), "the .py file is seeded");
        assert!(paths.contains(&"stack.ts"), "the .ts file is seeded");
        // The answer key rides along for the browser Check on the .ts buffer.
        assert!(!call.workbench.checks.is_empty(), "the step carries checks");
    }

    #[test]
    fn a_pure_config_step_parses_and_is_not_runnable() {
        // A configuration step: a single .ts file, checks, no responses. The
        // single editor opens the .ts file, the button is Check, and no
        // terminal appears because the step is not runnable.
        let workbench = serde_json::json!({
            "open": "stack.ts",
            "files": [{ "path": "stack.ts", "body": "// here\n" }],
            "checks": [{ "pattern": "Bucket", "present": true, "message": "a bucket" }]
        });
        let ts = serde_json::json!([
            { "label": "stack", "language": "typescript", "source": "const b = new Bucket();" }
        ]);
        let source = scenario_with(ts, workbench);
        let scenario = Scenario::parse(&source).expect("a pure config step parses");
        let call = scenario.steps[1].as_call().unwrap();

        assert!(!call.is_runnable(), "a config step runs no call");
        assert!(
            !call.workbench.checks.is_empty(),
            "a config step is graded by its checks"
        );
    }

    #[test]
    fn a_runnable_step_without_a_shell_snippet_is_rejected() {
        // A runnable step shows the always-on terminal, so it must teach a
        // command to type: at least one shell snippet. Python and a response
        // but no shell leaves the terminal with no lesson — the gap the audit
        // found — so the parser rejects it.
        let workbench = serde_json::json!({
            "open": "answer.py",
            "files": [{ "path": "answer.py", "body": "# here\n" }],
            "responses": [{ "service": "s", "operation": "Op", "body": {} }]
        });
        let py = serde_json::json!([
            { "label": "call", "language": "python", "source": "import boto3\nboto3.client(\"s\").op()" }
        ]);
        let source = scenario_with(py, workbench);
        let err = Scenario::parse(&source)
            .expect_err("a runnable step with no shell snippet is rejected");
        assert!(
            matches!(err, ScenarioError::RunnableStepHasNoShell { .. }),
            "expected RunnableStepHasNoShell, got {err:?}"
        );
    }
}
