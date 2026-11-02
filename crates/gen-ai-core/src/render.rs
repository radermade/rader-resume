//! Markup, rendered from typed data.
//!
//! These functions take a scenario and return `Markup`. No HTTP types, no AWS
//! types, no environment reads, which is what lets the same functions serve a
//! request now and produce a static artifact later.

use maud::{DOCTYPE, Markup, html};

use crate::assets::{NAV, WORKBENCH, WORKER};
use crate::diagram::{self, Layout};
use crate::scenario::{CallStep, Cursor, Scenario, Step};

const STYLES: &str = include_str!("styles.css");

/// The banner. Monospace, so it is authored as literal rows rather than
/// generated: a figlet at runtime would be a dependency for one string.
const BANNER: &str = r"  ___ _____ _  __
 / _ \___  | |/ /
| |_| | / /|   <
|  _  |/ / | |\ \
|_| |_/_/  |_| \_\";

/// The one place the lab's inlined stylesheet is assembled.
///
/// The authored CSS plus the diagram geometry for the scenario being
/// rendered. One block rather than two, so the policy carries one style hash.
fn stylesheet(layout: Option<&Layout<'_>>) -> String {
    match layout {
        Some(layout) => format!("{STYLES}{}", layout.css()),
        None => STYLES.to_owned(),
    }
}

/// The workbench's data, serialized for the browser asset to read.
///
/// A `<script type="application/json">` block, not a `<script>` that executes:
/// the data is inert, and the asset fetches it by id. That keeps the page
/// under a `script-src` that authorizes only the asset, with no inline code
/// to hash.
///
/// The one escape that matters is `</`. `maud` escapes text content, but a
/// literal `</script>` inside authored Python would close the block early
/// regardless of HTML escaping, because the HTML parser looks for that
/// sequence before it looks at entities. Neutralizing `</` to `<\/` is the
/// documented way to embed arbitrary text in a script element, and `<\/` is
/// identical to `</` once JSON-parsed.
fn workbench_json(call: &CallStep) -> String {
    // The workbench data plus the worker's content-hashed URL. The main thread
    // creates the worker from this URL rather than a Blob, so `worker-src`
    // stays `'self'` with no `blob:`, and the worker is a real served file
    // rather than a string. The data block is the one place the URL crosses
    // into the browser.
    let payload = serde_json::json!({
        "workbench": &call.workbench,
        "workerUrl": WORKER.url(),
        // Whether this step runs a live call. A configuration step seeds a
        // CDK/config artifact (TypeScript, YAML) to read and assemble, not
        // Python to execute. It does not run against the intercept; instead
        // its Run checks the assembled artifact against the structural checks
        // below. The JS branches on this.
        "runnable": call.is_runnable(),
        // The config step's answer key: checks run in the browser when Run is
        // pressed on a step with no live call. A runnable step has none.
        "checks": &call.workbench.checks,
    });
    let json = serde_json::to_string(&payload).expect("workbench serializes");
    json.replace("</", "<\\/")
}

/// A CSP source expression for an inline block, hashed as the spec requires.
fn csp_hash(source: &str) -> String {
    use base64::Engine as _;
    use sha2::Digest as _;

    let digest = sha2::Sha256::digest(source.as_bytes());
    let encoded = base64::engine::general_purpose::STANDARD.encode(digest);

    format!("'sha256-{encoded}'")
}

/// What a page needs the policy to allow beyond the baseline.
///
/// The overview and the landing need nothing but their own styles. A call
/// step mounts the workbench, which is where the real widening lives, so it is
/// confined to the pages that actually run code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PagePowers {
    /// Styles only. The landing and the overview.
    Static,
    /// The workbench: the asset, the Pyodide runtime, and the fetches that
    /// load it.
    Workbench,
}

/// The lab's Content Security Policy for a given page.
///
/// Its own policy, not the resume's, because the two apps inline different
/// stylesheets. Closed by default, opened only as far as the page in hand
/// needs: a reader on the overview is under a policy as tight as the resume's,
/// and the widening is paid only on a page that runs code.
///
/// Style is a hash rather than a nonce. A nonce must be unique per response,
/// which would make every page uncacheable and foreclose pre-rendering these
/// as static artifacts. The hash varies per page because the diagram geometry
/// is inside the block, which is the cost of generating geometry rather than
/// using `style` attributes, and cheaper than `'unsafe-hashes'`.
fn policy(layout: Option<&Layout<'_>>, powers: PagePowers) -> String {
    let mut directives = vec![
        "default-src 'none'".to_owned(),
        "base-uri 'none'".to_owned(),
        // Previous and Next post to the same origin.
        "form-action 'self'".to_owned(),
        format!("style-src {}", csp_hash(&stylesheet(layout))),
    ];

    // The architecture diagram draws product icons as `<img>` from this
    // origin, and only a page with a layout draws the diagram (the overview).
    // So `img-src 'self'` is added exactly where an icon can appear and
    // nowhere else, rather than as a blanket directive. Same-origin only; no
    // third party.
    if layout.is_some() {
        directives.push("img-src 'self'".to_owned());
    }

    if powers == PagePowers::Workbench {
        // Everything the workbench needs is on this origin, so the whole
        // policy stays `'self'` with no third-party host. Pyodide, its wasm,
        // and the wheels are vendored and served from `/gen-ai/assets/`, which
        // is what keeps `script-src 'self'` literally true and the lab to one
        // vendor.
        //
        // `wasm-unsafe-eval` is Pyodide's requirement for compiling the
        // runtime; it weakens the defence against injected script, not against
        // the visitor's own code, which is why the lab stores and echoes
        // nothing an injection could ride. `connect-src 'self'` lets the
        // worker fetch the runtime and the wheels, all co-located on the lab
        // origin. `worker-src 'self'` because the runtime thread is spawned
        // from a content-hashed worker file served from this origin, not a
        // Blob — so no `blob:` is needed, which is the payoff of serving the
        // worker as a real asset rather than an inline string.
        directives.push("script-src 'self' 'wasm-unsafe-eval'".to_owned());
        directives.push("connect-src 'self'".to_owned());
        directives.push("worker-src 'self'".to_owned());
    }

    directives.join("; ")
}

/// The policy for a page with no workbench. Kept public for the handler and
/// tests that render the landing or an overview.
pub fn content_security_policy(layout: Option<&Layout<'_>>) -> String {
    policy(layout, PagePowers::Static)
}

struct Head<'a> {
    title: &'a str,
    description: &'a str,
    powers: PagePowers,
}

fn shell(head: Head<'_>, layout: Option<&Layout<'_>>, body: Markup) -> Markup {
    let styles = stylesheet(layout);

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                // Before the style it governs. A policy parsed after the
                // content it authorizes does nothing for what is already
                // parsed.
                meta http-equiv="Content-Security-Policy"
                     content=(policy(layout, head.powers));
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="description" content=(head.description);
                title { (head.title) }
                style { (maud::PreEscaped(styles)) }
            }
            body { (body) }
        }
    }
}

/// `/gen-ai`: the split. Banner on the left, projects on the right.
pub fn landing(catalog: &[Scenario]) -> Markup {
    let body = html! {
        div class="lab-landing" {
            // Decorative: the heading on the right names the page. A screen
            // reader does not need the ASCII art spelled out.
            div class="lab-banner" {
                pre aria-hidden="true" {
                    (BANNER)
                    "\n"
                }
                p class="lab-prompt" aria-hidden="true" {
                    "radermade@gen-ai:~$ ls projects"
                }
            }

            main class="lab-projects" {
                h1 { "AWS GenAI Lab" }
                ul {
                    @for scenario in catalog {
                        li class="lab-project" {
                            a href=(format!("/gen-ai/{}", scenario.id)) {
                                (scenario.name)
                            }
                            p { (scenario.summary) }
                        }
                    }
                }
            }
        }
    };

    shell(
        Head {
            title: "AWS GenAI Lab",
            description: "Walk a generative AI build on AWS, one call at a time.",
            powers: PagePowers::Static,
        },
        None,
        body,
    )
}

/// The job. Stays on screen for the whole project.
fn brief(scenario: &Scenario) -> Markup {
    let brief = &scenario.brief;

    html! {
        section class="lab-brief" {
            div {
                span class="lab-brief__label" { "Who is asking" }
                p { (brief.who_is_asking) }
            }
            div {
                span class="lab-brief__label" { "What they need back" }
                p { (brief.what_they_need_back) }
            }
            div {
                span class="lab-brief__label" { "Already running" }
                p { (brief.systems_already_running) }
            }
            // Marked, because it is the thing every step argues against
            // rather than one more paragraph of context.
            div class="lab-brief__breaks" {
                span class="lab-brief__label" { "Breaks when" }
                p { (brief.breaks_when) }
            }
            div {
                span class="lab-brief__label" { "Done means" }
                p { (brief.done_means) }
            }
        }
    }
}

/// The interior of the lesson region.
///
/// Shared by the full page and the swap endpoint, so a step rendered either
/// way is byte-identical. The `data-workbench` attribute tells the swap script
/// whether this step wants the workbench, which is also whether it is under
/// the widened policy: a transition that changes that answer crosses a CSP
/// boundary a fragment swap cannot, so the script falls back to a full
/// navigation there. Overview-to-first-call is the only such transition.
pub fn lesson(cursor: &Cursor<'_>, layout: Option<&Layout<'_>>) -> Markup {
    let scenario = cursor.scenario();
    let step = cursor.step();
    let has_workbench = step.has_console();

    html! {
        div
            class="lab-lesson__inner"
            data-step=(cursor.index())
            data-workbench=(has_workbench)
        {
            h1 { (scenario.name) }
            p class="lab-step-title" {
                "Step " (cursor.number()) " of " (scenario.len())
                " · " (step.title())
            }

            (brief(scenario))
            (step_index(cursor))

            @match step {
                Step::Overview { .. } => {
                    @if let Some(layout) = layout {
                        (architecture(layout))
                    }
                }
                Step::Call(call) => {
                    // The step's workbench payload, carried inside the
                    // swappable region so a fragment swap can re-sync the
                    // persistent workbench to the new step. On a full load the
                    // canonical block in `workbench()` is what the asset reads;
                    // this carrier exists for the nav script, under a distinct
                    // id so the two never collide. Without it, Next swaps the
                    // lesson but leaves the workbench — its seed files, its
                    // config-step checks, its runnable flag — showing the
                    // previous step.
                    script type="application/json" id="lab-step-data" {
                        (maud::PreEscaped(workbench_json(call)))
                    }
                    section class="lab-pane" {
                        h2 { "The decision" }
                        p { (call.decision) }
                    }
                    section class="lab-pane" {
                        h2 { "Why" }
                        p { (call.why) }
                    }
                    // The pieces to assemble in the editor, each labelled.
                    // Reading them is the prompt; writing them is the practice,
                    // so they are shown rather than seeded into the file.
                    section class="lab-pane" {
                        h2 { "Write the call" }
                        p class="lab-pane__hint" {
                            "Assemble these in the editor on the right, then Run."
                        }
                        @for snippet in &call.snippets {
                            // A snippet in a runnable step that is not the open
                            // file's language is reference, not something to
                            // paste into the editor: the shell sidecar beside a
                            // PDF, a CDK trigger shown for context. Marking it
                            // keeps a visitor from assembling it into a .py file
                            // and hitting a syntax error on Run — the failure
                            // that reached step 9's editor.
                            @let is_reference = call.is_runnable()
                                && snippet.language != "python";
                            div class={
                                @if is_reference { "lab-snippet lab-snippet--reference" }
                                @else { "lab-snippet" }
                            } {
                                p class="lab-snippet__label" {
                                    span class="lab-snippet__lang" { (snippet.language) }
                                    (snippet.label)
                                    @if is_reference {
                                        span class="lab-snippet__ref" {
                                            " — reference, not for the editor"
                                        }
                                    }
                                }
                                pre class="lab-code" {
                                    code { (snippet.source) }
                                }
                            }
                        }
                    }
                    section class="lab-pane" {
                        h2 { "Infrastructure" }
                        pre class="lab-code" {
                            code { (call.infrastructure) }
                        }
                    }
                }
            }

            form class="lab-nav" method="post" action=(format!("/gen-ai/{}", scenario.id)) {
                input type="hidden" name="step" value=(cursor.index());
                button type="submit" name="go" value="previous" disabled[cursor.is_first()] {
                    "Previous"
                }
                button type="submit" name="go" value="next" disabled[cursor.is_last()] {
                    "Next"
                }
                a href="/gen-ai" { "All projects" }
            }
        }
    }
}

fn step_index(cursor: &Cursor<'_>) -> Markup {
    html! {
        ol class="lab-index" {
            @for (index, step) in cursor.scenario().steps.iter().enumerate() {
                @if index == cursor.index() {
                    li aria-current="step" { (index + 1) ". " (step.title()) }
                } @else {
                    li { (index + 1) ". " (step.title()) }
                }
            }
        }
    }
}

fn architecture(layout: &Layout<'_>) -> Markup {
    let view_box = format!("0 0 {:.0} {:.0}", layout.width, layout.height);

    html! {
        div class="lab-diagram" {
            div class="lab-diagram__frame" {
                // Connectors only. SVG is right for curves between computed
                // points and wrong for wrapping text, so nodes are HTML on
                // top of this.
                svg class="lab-diagram__canvas"
                    viewBox=(view_box)
                    preserveAspectRatio="none"
                    aria-hidden="true"
                    focusable="false"
                {
                    g class="lab-diagram__edges" {
                        @for connector in &layout.connectors {
                            path d=(connector.path()) {}
                        }
                    }
                }

                @for group in &layout.groups {
                    div class="lab-group" id=(format!("lab-group-{}", group.id)) {
                        span class="lab-group__label" { (group.label) }
                    }
                }

                @for placed in &layout.nodes {
                    div class="lab-node" id=(format!("lab-node-{}", placed.node.id)) {
                        // The product icon, when the node names one the lab
                        // has. aria-hidden: the caption below is the label, so
                        // the diagram reads identically with or without it,
                        // and an unknown key simply renders no image.
                        @if let Some(url) = placed.node.icon.as_deref().and_then(crate::assets::icon_url) {
                            img class="lab-node__icon" src=(url) alt="" aria-hidden="true";
                        }
                        span class="lab-node__caption" { (placed.node.label) }
                    }
                }
            }
        }
    }
}

/// `/gen-ai/{id}`: one step.
///
/// Previous and Next are submit buttons in a form posting the current step, so
/// navigation works with no JavaScript. The console arrives in phase 2 and the
/// overview never gets one, which is a property of the step's type rather than
/// a flag on this function.
pub fn step_page(cursor: &Cursor<'_>) -> Markup {
    let scenario = cursor.scenario();
    let step = cursor.step();

    // Computed once and shared: the markup needs the positions and the policy
    // needs the hash of the CSS they generate.
    let layout = match step {
        Step::Overview { diagram, .. } => Some(diagram::layout(diagram)),
        Step::Call(_) => None,
    };

    // One column on the overview, two from a call step. The grid modifier is
    // what the workbench column keys off, and it is absent on the overview so
    // there is no empty second column to style around.
    let step_class = match step {
        Step::Overview { .. } => "lab-step",
        Step::Call(_) => "lab-step lab-step--working",
    };

    let body = html! {
        div class=(step_class) {
            // The swap target. Next replaces this region's contents and
            // nothing else, which is how the workbench beside it keeps its
            // files. Its interior is the `lesson` fragment, shared with the
            // swap endpoint so the two cannot diverge.
            main class="lab-lesson" id="lab-lesson" {
                (lesson(cursor, layout.as_ref()))
            }

            // The workbench is a sibling of the swap target, not inside it, so
            // Next leaves it mounted with its files. Rendered only on a call
            // step: the overview has none, which `as_call` makes a type fact
            // rather than a flag.
            @if let Some(call) = step.as_call() {
                (workbench(call))
            }
        }
    };

    // The widened policy is paid only where the workbench is. An overview is
    // as locked down as the landing.
    let powers = match step {
        Step::Overview { .. } => PagePowers::Static,
        Step::Call(_) => PagePowers::Workbench,
    };

    shell(
        Head {
            title: scenario.name.as_str(),
            description: scenario.summary.as_str(),
            powers,
        },
        layout.as_ref(),
        body,
    )
}

/// The workbench column: the step's seed data for the asset, and the shell
/// the asset mounts into.
///
/// The Rust side renders the data and the empty containers. Everything live —
/// the runtime, the file tree, the editor, running code — is the browser
/// asset's job, and without it this degrades to a readable starting file
/// rather than a broken pane.
fn workbench(call: &CallStep) -> Markup {
    html! {
        section class="lab-workbench" aria-label="Workbench" {
            // Inert data the asset reads by id. Not executable script.
            script type="application/json" id="lab-workbench-data" {
                (maud::PreEscaped(workbench_json(call)))
            }

            // The no-script fallback: the file that would be open, readable,
            // so the step still teaches with the runtime absent or still
            // loading. The asset replaces this with the live editor.
            noscript {
                pre class="lab-code" {
                    code {
                        @for file in &call.workbench.files {
                            @if file.path == call.workbench.open {
                                (file.body)
                            }
                        }
                    }
                }
            }

            div class="lab-files" {}
            div class="lab-editor" {}
            div class="lab-output" {}
            div class="lab-cli" {}
        }

        // Both loaded from content-hashed URLs so `script-src 'self'` covers
        // them with no inline code to hash. `defer` so they run after the
        // data block and the containers are parsed. Only on a call step: a
        // reader on the overview fetches neither, and the overview's
        // style-only policy would block them anyway.
        script src=(WORKBENCH.url()) defer {}
        script src=(NAV.url()) defer {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Vec<Scenario> {
        crate::catalog().unwrap()
    }

    /// The index of the first runnable call step in a scenario — one that
    /// intercepts a response and so exercises boto3. Tests that assert on a
    /// live call find it this way rather than hardcoding an index, so
    /// inserting configuration steps ahead of it does not break them.
    fn first_runnable(scenario: &Scenario) -> usize {
        scenario
            .steps
            .iter()
            .position(|s| s.as_call().is_some_and(|c| c.is_runnable()))
            .expect("a scenario has at least one runnable call step")
    }

    #[test]
    fn landing_lists_every_project_as_a_link() {
        let catalog = catalog();
        let html = landing(&catalog).into_string();

        assert!(html.starts_with("<!DOCTYPE html>"));

        for scenario in &catalog {
            assert!(html.contains(&scenario.name));
            assert!(html.contains(&scenario.summary));
            assert!(html.contains(&format!(r#"href="/gen-ai/{}""#, scenario.id)));
        }
    }

    #[test]
    fn the_banner_is_hidden_from_assistive_technology() {
        let html = landing(&catalog()).into_string();

        // ASCII art read aloud is noise. The heading names the page.
        assert!(html.contains(r#"<pre aria-hidden="true">"#));
        assert!(html.contains("AWS GenAI Lab"));
    }

    #[test]
    fn the_overview_has_no_console_and_draws_the_system() {
        let catalog = catalog();
        let cursor = Cursor::at(&catalog[0], 0);
        let html = step_page(&cursor).into_string();

        assert!(!cursor.step().has_console());
        // Phase 2 introduces the console. Nothing on the overview should hint
        // at one existing.
        assert!(!html.contains("lab-console"));

        assert!(html.contains("<svg"));
        assert!(html.contains("lab-node-lambda"));
        assert!(html.contains("Bedrock Knowledge Bases"));
        assert!(html.contains("EU Region"));
    }

    #[test]
    fn a_call_step_shows_its_panes_and_a_workbench() {
        let catalog = catalog();
        let cursor = Cursor::at(&catalog[0], 1);
        let html = step_page(&cursor).into_string();

        assert!(cursor.step().has_console());
        assert!(html.contains("The decision"));
        assert!(html.contains("Why"));
        assert!(html.contains("Infrastructure"));

        // The snippets to assemble are shown in the walkthrough, with the
        // instruction to write them into the editor. The step does not ship
        // the finished file.
        assert!(html.contains("Write the call"));
        let call = cursor.step().as_call().unwrap();
        for snippet in &call.snippets {
            assert!(html.contains(&snippet.label), "snippet label missing");
        }

        // The workbench mounts beside the lesson.
        assert!(html.contains("lab-workbench"));
        assert!(html.contains(r#"id="lab-workbench-data""#));
        assert!(html.contains("lab-step--working"));
    }

    #[test]
    fn the_lesson_fragment_carries_its_policy_class_and_matches_the_page() {
        let catalog = catalog();
        let cursor = Cursor::at(&catalog[0], 1);
        let layout = None;

        let fragment = lesson(&cursor, layout).into_string();

        // The swap script reads data-workbench to decide whether a transition
        // crosses the CSP boundary. A call step wants the workbench.
        assert!(fragment.contains(r#"data-workbench="true""#));

        // The fragment is the interior the full page also renders, so the two
        // cannot diverge: the page contains the fragment verbatim.
        let page = step_page(&cursor).into_string();
        assert!(page.contains(&fragment));

        // A fragment is not a document: no shell, no policy of its own.
        assert!(!fragment.contains("<!DOCTYPE"));
        assert!(!fragment.contains("Content-Security-Policy"));
    }

    #[test]
    fn the_lesson_fragment_carries_the_steps_own_workbench_payload() {
        // The swap replaces only the lesson, leaving the workbench mounted. So
        // the fragment must carry its step's workbench payload for the nav
        // script to re-sync, or a config step grades against the previous
        // step's checks. Find two adjacent config steps and show each
        // fragment carries its own checks, not its neighbour's.
        let catalog = catalog();
        let scenario = &catalog[0];

        let config_indices: Vec<usize> = scenario
            .steps
            .iter()
            .enumerate()
            .filter(|(_, s)| s.as_call().is_some_and(|c| !c.is_runnable()))
            .map(|(i, _)| i)
            .collect();
        assert!(
            config_indices.len() >= 2,
            "need two config steps to prove the fragment is step-specific"
        );

        let a = Cursor::at(scenario, config_indices[0]);
        let b = Cursor::at(scenario, config_indices[1]);
        let a_call = a.step().as_call().unwrap();
        let b_call = b.step().as_call().unwrap();

        let frag_a = lesson(&a, None).into_string();
        let frag_b = lesson(&b, None).into_string();

        let _ = a_call; // kept for symmetry; the assertion is on the fragments.

        assert!(
            frag_a.contains(r#"id="lab-step-data""#),
            "the fragment must carry the step's workbench payload for re-sync"
        );

        // Pick a check message unique to step B, not present anywhere in A's
        // fragment (snippets and infrastructure included). A swap that failed
        // to re-sync would be the only way B's answer key could surface on A.
        let b_only = b_call
            .workbench
            .checks
            .iter()
            .map(|c| c.message.as_str())
            .find(|m| !frag_a.contains(*m))
            .expect("two different config steps should differ in at least one check message");

        assert!(
            frag_b.contains(b_only),
            "step B's fragment must carry step B's own checks"
        );
        assert!(
            !frag_a.contains(b_only),
            "step A's fragment must not carry step B's checks"
        );
    }

    #[test]
    fn the_overview_fragment_declares_no_workbench() {
        let catalog = catalog();
        let cursor = Cursor::at(&catalog[0], 0);
        let layout = match cursor.step() {
            Step::Overview { diagram, .. } => diagram::layout(diagram),
            Step::Call(_) => unreachable!(),
        };

        let fragment = lesson(&cursor, Some(&layout)).into_string();

        // The overview is under the style-only policy. The script sees this
        // differ from a call step's `true` and falls back to a full
        // navigation, which is the only way to cross the CSP boundary.
        assert!(fragment.contains(r#"data-workbench="false""#));
    }

    #[test]
    fn both_scripts_load_from_hashed_urls_on_a_call_step() {
        let catalog = catalog();
        let call = step_page(&Cursor::at(&catalog[0], 1)).into_string();

        assert!(call.contains(&format!(r#"<script src="{}" defer>"#, WORKBENCH.url())));
        assert!(call.contains(&format!(r#"<script src="{}" defer>"#, NAV.url())));
    }

    #[test]
    fn a_call_step_loads_the_asset_and_the_overview_does_not() {
        let catalog = catalog();

        let call = step_page(&Cursor::at(&catalog[0], 1)).into_string();
        let overview = step_page(&Cursor::at(&catalog[0], 0)).into_string();

        // Loaded by src from the content-hashed URL, so `script-src 'self'`
        // covers it with no inline code. `defer` so it runs after the data
        // block is parsed.
        assert!(call.contains(&format!(r#"<script src="{}" defer>"#, WORKBENCH.url())));

        // A reader on the overview never fetches the runtime scripts. It does
        // fetch diagram icons from `/gen-ai/assets/`, so the check is on the
        // scripts specifically, not any asset: no workbench, worker, or nav
        // script, and no `<script src>` at all on a style-only page.
        assert!(!overview.contains(&WORKBENCH.url()));
        assert!(!overview.contains(&WORKER.url()));
        assert!(!overview.contains(&NAV.url()));
        assert!(!overview.contains("<script src"));
    }

    #[test]
    fn the_overview_draws_product_icons_from_this_origin() {
        let catalog = catalog();
        let overview = step_page(&Cursor::at(&catalog[0], 0)).into_string();

        // The diagram nodes that name an icon render it as an aria-hidden img
        // from a content-hashed same-origin URL, so the caption stays the
        // label and `img-src 'self'` covers the fetch.
        let lambda = crate::assets::icon_url("lambda").unwrap();
        assert!(overview.contains(&format!(r#"<img class="lab-node__icon" src="{lambda}""#)));
        assert!(overview.contains(r#"alt="" aria-hidden="true""#));
        // The page declares img-src so the icon is allowed.
        assert!(overview.contains("img-src 'self'"));
    }

    #[test]
    fn the_workbench_data_is_inert_and_escaped() {
        let catalog = catalog();
        let cursor = Cursor::at(&catalog[0], first_runnable(&catalog[0]));
        let html = step_page(&cursor).into_string();

        // A JSON data block, not executable script, so no inline code needs a
        // hash and the policy can keep script-src to the asset alone.
        assert!(html.contains(r#"<script type="application/json" id="lab-workbench-data">"#));

        // The seed content is present, so the asset has something to mount.
        // Assert on the open file's path, which every step's data block
        // carries, rather than a token that happens to be in one step's seed.
        let call = cursor.step().as_call().unwrap();
        assert!(html.contains(&call.workbench.open));

        // No raw `</script>` from authored content can close the block early.
        // The serializer neutralizes `</`, so the only `</script>` in the
        // document is the real closing tag the template emits.
        assert_eq!(
            html.matches("</script>").count(),
            html.matches("<script").count(),
            "an unescaped </ in authored content closed a script block early"
        );
    }

    #[test]
    fn the_serializer_neutralizes_a_closing_script_tag() {
        use crate::scenario::{Check, Intercepted, Reason, SeedFile, Snippet, Workbench};

        // A hostile seed file that contains the one sequence that would break
        // out of a JSON script block. Authored content is trusted, but a
        // policy document pasted verbatim could contain this by accident.
        let call = CallStep {
            title: "t".into(),
            decision: "d".into(),
            why: "w".into(),
            snippets: vec![Snippet {
                label: "l".into(),
                language: "python".into(),
                source: "x".into(),
            }],
            infrastructure: "i".into(),
            workbench: Workbench {
                open: "a.py".into(),
                files: vec![SeedFile {
                    path: "a.py".into(),
                    body: "s = \"</script><b>pwned</b>\"".into(),
                }],
                responses: vec![Intercepted {
                    service: "s".into(),
                    operation: "O".into(),
                    body: serde_json::json!({}),
                }],
                reasons: Vec::<Reason>::new(),
                checks: Vec::<Check>::new(),
            },
        };

        let json = workbench_json(&call);
        assert!(!json.contains("</script>"));
        assert!(json.contains("<\\/script>"));

        // And it round-trips: the escape is cosmetic to the HTML parser and
        // identical to `</` once JSON decodes it. The data block wraps the
        // workbench alongside the worker URL, so the files live under
        // `workbench`.
        let decoded: serde_json::Value = serde_json::from_str(&json.replace("<\\/", "</")).unwrap();
        assert_eq!(
            decoded["workbench"]["files"][0]["body"],
            "s = \"</script><b>pwned</b>\""
        );
    }

    #[test]
    fn a_call_step_still_teaches_without_script() {
        let catalog = catalog();
        let cursor = Cursor::at(&catalog[0], first_runnable(&catalog[0]));
        let html = step_page(&cursor).into_string();
        let call = cursor.step().as_call().unwrap();

        // The teaching content is the snippets, and they are server-rendered
        // in the walkthrough, so the step teaches with no runtime at all. The
        // source is HTML-escaped in the page, so assert on a token that
        // survives escaping rather than the raw source, plus every label.
        for snippet in &call.snippets {
            assert!(
                html.contains(&snippet.label),
                "a snippet label is missing from the server-rendered page"
            );
        }
        // A distinctive identifier from the call survives escaping and proves
        // the code itself, not just the labels, reached the page.
        assert!(html.contains("bedrock-agent-runtime"));
        assert!(html.contains("knowledgeBaseId"));

        // The workbench shows its seeded file in a noscript fallback rather
        // than a blank pane, so the scaffold is visible before the asset
        // mounts.
        assert!(html.contains("<noscript>"));
    }

    #[test]
    fn the_job_is_on_every_step() {
        let catalog = catalog();

        for index in 0..catalog[0].len() {
            let html = step_page(&Cursor::at(&catalog[0], index)).into_string();

            // A step that is only a product name is not a step. The brief is
            // what makes it a decision, so it stays visible throughout.
            assert!(html.contains("Breaks when"));
            assert!(html.contains(&catalog[0].brief.breaks_when));
            assert!(html.contains(&catalog[0].brief.done_means));
        }
    }

    #[test]
    fn navigation_works_without_script() {
        let catalog = catalog();
        let cursor = Cursor::at(&catalog[0], 0);
        let html = step_page(&cursor).into_string();

        // A form post, so phase 2's htmx swap is an upgrade to something that
        // already functions.
        assert!(html.contains(r#"method="post""#));
        assert!(html.contains(r#"name="step" value="0""#));
        assert!(html.contains(r#"value="next""#));
        // Previous is disabled on the first step rather than absent, so the
        // control does not move between steps.
        assert!(html.contains("disabled"));
    }

    #[test]
    fn the_last_step_cannot_advance() {
        let catalog = catalog();
        let last = catalog[0].len() - 1;
        let cursor = Cursor::at(&catalog[0], last);

        assert!(cursor.is_last());
        let html = step_page(&cursor).into_string();
        assert!(html.contains(r#"value="next" disabled"#));
    }

    #[test]
    fn the_overview_policy_is_as_tight_as_the_resume() {
        let catalog = catalog();

        // The overview inlines generated geometry, so its policy must hash the
        // stylesheet including that. A policy built from the authored CSS
        // alone would block the page's own styles.
        let layout = match Cursor::at(&catalog[0], 0).step() {
            Step::Overview { diagram, .. } => diagram::layout(diagram),
            Step::Call(_) => unreachable!("step 1 is the overview"),
        };

        let overview_policy = policy(Some(&layout), PagePowers::Static);
        assert!(overview_policy.contains(&csp_hash(&stylesheet(Some(&layout)))));
        assert!(overview_policy.starts_with("default-src 'none'"));
        // A reader who never runs code is under no script or eval power at all.
        assert!(!overview_policy.contains("script-src"));
        assert!(!overview_policy.contains("unsafe-inline"));
        assert!(!overview_policy.contains("unsafe-eval"));
        assert!(!overview_policy.contains("unsafe-hashes"));
        assert!(!overview_policy.contains('*'));
        // The one widening the overview takes over the landing is `img-src
        // 'self'`, for the diagram icons — same origin, no script power.
        assert!(overview_policy.contains("img-src 'self'"));

        // And the landing, which draws no diagram, does not even take that:
        // no layout means no img-src.
        let landing_policy = policy(None, PagePowers::Static);
        assert!(!landing_policy.contains("img-src"));
    }

    #[test]
    fn the_workbench_policy_widens_only_as_far_as_pyodide_needs() {
        let policy = policy(None, PagePowers::Workbench);

        // The asset is a file on this origin, so `'self'`; `wasm-unsafe-eval`
        // is Pyodide's, and `connect-src`/`worker-src` load and run it.
        assert!(policy.contains("script-src 'self' 'wasm-unsafe-eval'"));
        assert!(policy.contains("connect-src 'self'"));
        // The worker is served from this origin, so worker-src is plain
        // `'self'` with no `blob:` — nothing in the policy needs a blob source
        // now that the worker is a real file rather than an inline string.
        assert!(policy.contains("worker-src 'self'"));
        assert!(!policy.contains("blob:"));

        // The widening stops there. No inline script, no eval beyond wasm, no
        // wildcard. `wasm-unsafe-eval` is the one concession and it is named.
        assert!(!policy.contains("unsafe-inline"));
        assert!(!policy.contains("unsafe-hashes"));
        assert!(
            !policy
                .replace("wasm-unsafe-eval", "")
                .contains("unsafe-eval")
        );
        assert!(!policy.contains('*'));
    }

    #[test]
    fn the_rendered_policy_matches_the_step_kind() {
        let catalog = catalog();

        // The overview page carries no script-src; the call page does. The
        // widening is confined to pages that actually run code, which is the
        // whole point of threading the power through rather than applying one
        // policy to every page.
        let overview = step_page(&Cursor::at(&catalog[0], 0)).into_string();
        let call = step_page(&Cursor::at(&catalog[0], 1)).into_string();

        assert!(!overview.contains("wasm-unsafe-eval"));
        assert!(call.contains("wasm-unsafe-eval"));
    }

    #[test]
    fn no_page_positions_itself_with_inline_styles() {
        let catalog = catalog();

        for index in 0..catalog[0].len() {
            let html = step_page(&Cursor::at(&catalog[0], index)).into_string();

            // A style hash does not cover `style` attributes. Using them would
            // have needed 'unsafe-hashes', so geometry is generated into the
            // hashed block instead.
            assert!(
                !html.contains("style=\""),
                "step {index} positions itself with an attribute the policy blocks"
            );
        }

        assert!(!landing(&catalog).into_string().contains("style=\""));
    }

    #[test]
    fn the_lab_shares_no_token_with_the_resume() {
        // Separate palettes mean separate custom properties. A `--surface`
        // here would be the start of the two stylesheets drifting into each
        // other.
        assert!(STYLES.contains("--lab-prompt"));
        assert!(!STYLES.contains("--surface-sunken"));
        assert!(!STYLES.contains("--accent-warm"));
    }
}
