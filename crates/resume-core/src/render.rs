//! Markup, rendered from typed data.
//!
//! These functions take content and return `Markup`. No AWS types, no HTTP
//! types, no environment reads. That is what lets the same functions run in a
//! request handler now and in a pipeline render task later, which is the whole
//! point of open question 7 resolving to static-first.

use maud::{DOCTYPE, Markup, html};

use crate::architecture::{Architecture, MANIFESTS, Node};
use crate::fit::{Assessment, Dimension, FitReport, Requirement, RequirementKind, Verdict};
use crate::provenance::{Check, CheckRun, Outcome};
use crate::{CareerBreak, ImpreciseDate, Project, Resume, Role, SkillGroup, TimelineEntry};

/// Styles are inlined. One request, well inside the page weight budget, and no
/// render-blocking stylesheet fetch.
const STYLES: &str = include_str!("styles.css");

/// The stylesheet as inlined, which is the authored CSS plus the diagram's
/// generated geometry.
///
/// One block rather than two, so the policy carries one style hash and is
/// byte-identical on every page. That matters for the static-artifact target:
/// a per-page policy would have to be recomputed wherever a page is rendered.
///
/// `expect` because the architecture document is embedded at compile time and
/// a test asserts it parses. A malformed document is a build-time bug, and
/// failing loudly beats serving a diagram with every node stacked at the
/// origin.
static INLINE_STYLES: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    let architecture =
        crate::architecture::load().expect("embedded architecture document must be valid");

    format!("{STYLES}{}", architecture.layout_css())
});

/// The only script on the site, inlined, and the sole reason `script-src` is
/// not `'none'`.
///
/// A constant rather than a literal inside the template, because the Content
/// Security Policy hashes exactly these bytes. Two copies would drift, and the
/// failure mode of a drifted hash is the modal silently not opening, which is
/// the kind of bug that survives a long time.
const ENHANCEMENT: &str = r#"
(() => {
  const dialog = document.getElementById('fit-dialog');
  const trigger = document.getElementById('fit-trigger');
  if (!dialog || !trigger || typeof dialog.showModal !== 'function') return;

  trigger.addEventListener('click', (event) => {
    event.preventDefault();
    dialog.showModal();
  });

  const field = dialog.querySelector('textarea');
  const stop = (event) => event.preventDefault();

  for (const name of ['dragenter', 'dragover']) {
    field.addEventListener(name, (event) => {
      stop(event);
      field.classList.add('is-dropping');
    });
  }
  for (const name of ['dragleave', 'drop']) {
    field.addEventListener(name, () => field.classList.remove('is-dropping'));
  }

  // Dropped text only. Files would mean parsing untrusted binaries, which is
  // deliberately out of scope.
  field.addEventListener('drop', (event) => {
    stop(event);
    const text = event.dataTransfer?.getData('text/plain');
    if (text) field.value = text;
  });

  // Render the report into the dialog instead of navigating away. The point
  // of the modal is that a visitor reading the resume stays on it.
  const panel = document.getElementById('fit-dialog-panel');
  const result = document.getElementById('fit-dialog-result');
  const again = document.getElementById('fit-dialog-again');
  const form = field.closest('form');
  if (!panel || !result || !again || !form) return;

  form.addEventListener('submit', async (event) => {
    event.preventDefault();

    const submit = form.querySelector('button[type="submit"]');
    if (submit) submit.disabled = true;
    dialog.setAttribute('aria-busy', 'true');

    try {
      const response = await fetch('/fit/fragment', {
        method: 'POST',
        body: new URLSearchParams(new FormData(form)),
        headers: { 'Accept': 'text/html' },
      });
      if (!response.ok) throw new Error(String(response.status));

      result.innerHTML = await response.text();
      panel.hidden = true;
      result.hidden = false;
      again.hidden = false;
      result.focus();
    } catch {
      // Anything unexpected falls back to the plain form post, which is a
      // full page but always works. Better a navigation than a dead button.
      form.submit();
    } finally {
      if (submit) submit.disabled = false;
      dialog.removeAttribute('aria-busy');
    }
  });

  again.querySelector('button').addEventListener('click', () => {
    result.innerHTML = '';
    result.hidden = true;
    again.hidden = true;
    panel.hidden = false;
    field.focus();
  });
})();
"#;

/// A CSP source expression for an inline block, hashed as the spec requires.
fn csp_hash(source: &str) -> String {
    use base64::Engine as _;
    use sha2::Digest as _;

    let digest = sha2::Sha256::digest(source.as_bytes());
    let encoded = base64::engine::general_purpose::STANDARD.encode(digest);

    format!("'sha256-{encoded}'")
}

/// The Content Security Policy, emitted into every document.
///
/// Hashes rather than a nonce. A nonce has to be unique per response, which
/// would make every page uncacheable and foreclose the pre-rendered static
/// artifacts that are the whole target for reads. Hashes are deterministic, so
/// the same policy is correct in a handler response and in a file on S3.
///
/// Computed from the same constants the document inlines, so it cannot drift
/// from what it authorizes.
///
/// `default-src 'none'` is the baseline, with each directive opened only as
/// far as something actually needs:
///
/// - `style-src` and `script-src` allow exactly the two inline blocks on this
///   site and nothing else, including nothing from any origin.
/// - `form-action 'self'` because the fit form posts to `/fit`.
/// - `img-src` allows this origin for the portrait that Phase 3 will add.
/// - `connect-src 'self'` for the one fetch on the site, which posts a job
///   description to the fragment endpoint so the report can render in the
///   dialog. Same origin only, so this does not permit sending that text
///   anywhere else.
///
/// Delivered as a `meta` element rather than a header, which is a real
/// tradeoff: `frame-ancestors`, `report-uri`, and `sandbox` are ignored in
/// `meta` and have to come from the edge. Everything above works, and in
/// exchange the policy travels with the document, so a pre-rendered artifact
/// carries its own protection without a CloudFront response-headers policy
/// having to stay in sync with it. Clickjacking protection belongs at the edge
/// alongside HSTS regardless.
pub fn content_security_policy() -> String {
    [
        "default-src 'none'".to_owned(),
        "base-uri 'none'".to_owned(),
        "form-action 'self'".to_owned(),
        "connect-src 'self'".to_owned(),
        "img-src 'self' data:".to_owned(),
        format!("style-src {}", csp_hash(&INLINE_STYLES)),
        format!("script-src {}", csp_hash(ENHANCEMENT)),
    ]
    .join("; ")
}

/// Everything the renderer needs that does not come from the content document.
///
/// A struct rather than globals so `resume-core` stays free of build-time
/// environment reads and the tests stay deterministic.
#[derive(Debug, Clone, Copy)]
pub struct PageContext<'a> {
    /// Commit the artifact was built from, for provenance.
    pub commit: &'a str,
    /// Reference date for derived values.
    pub as_of: ImpreciseDate,
    /// Canonical origin, e.g. `https://example.com`, with no trailing slash.
    ///
    /// `None` until a domain exists. Canonical and Open Graph URL tags are
    /// omitted rather than emitted as relative or placeholder values, because a
    /// wrong canonical is worse for SEO than a missing one.
    pub site_url: Option<&'a str>,
    /// Whether the provenance check is available.
    ///
    /// False where the run store is per-instance, which is everywhere except
    /// local development until the state machine lands. Offering a button that
    /// leads to an intermittent 404 is worse than not offering it.
    pub provenance_enabled: bool,
}

fn social_meta(resume: &Resume, ctx: PageContext<'_>) -> Markup {
    let profile = &resume.profile;
    let title = format!("{} — {}", profile.full_name, profile.headline);

    html! {
        meta property="og:type" content="profile";
        meta property="og:title" content=(title);
        meta property="og:description" content=(profile.summary);
        meta name="twitter:card" content="summary";

        @if let Some(origin) = ctx.site_url {
            link rel="canonical" href=(origin);
            meta property="og:url" content=(origin);
        }
    }
}

/// `robots.txt`, pointing crawlers at the sitemap when an origin is known.
pub fn robots_txt(site_url: Option<&str>) -> String {
    let mut out = String::from("User-agent: *\nAllow: /\n");
    if let Some(origin) = site_url {
        out.push_str(&format!("Sitemap: {origin}/sitemap.xml\n"));
    }
    out
}

/// `sitemap.xml`. Returns `None` without an origin, since sitemap entries must
/// be absolute URLs and a relative one is invalid rather than merely unhelpful.
pub fn sitemap_xml(site_url: Option<&str>) -> Option<String> {
    let origin = site_url?;
    Some(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
         \x20 <url><loc>{origin}/</loc></url>\n\
         </urlset>\n"
    ))
}

/// Per-page head values.
struct Head<'a> {
    title: &'a str,
    description: &'a str,
    /// Seconds until the page reloads itself.
    ///
    /// A `meta refresh` is the polling mechanism that works with JavaScript
    /// disabled. htmx can replace it with a partial swap as an enhancement;
    /// this is the baseline that always works.
    refresh: Option<u8>,
    /// Social tags belong on the resume page, not on transient run pages.
    social: bool,
}

fn shell(resume: &Resume, ctx: PageContext<'_>, head: Head<'_>, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                // Before the style and script it governs. A policy that
                // appears after the content it authorizes is ignored for
                // anything already parsed.
                meta http-equiv="Content-Security-Policy" content=(content_security_policy());
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="description" content=(head.description);
                @if let Some(seconds) = head.refresh {
                    meta http-equiv="refresh" content=(seconds);
                }
                title { (head.title) }
                @if head.social {
                    (social_meta(resume, ctx))
                }
                style { (maud::PreEscaped(INLINE_STYLES.as_str())) }
            }
            body {
                main { (body) }
                (footer(resume, ctx))
            }
        }
    }
}

pub fn page(resume: &Resume, ctx: PageContext<'_>) -> Markup {
    let profile = &resume.profile;
    let title = format!("{} — {}", profile.full_name, profile.headline);

    let body = html! {
        // The header spans the full content width across the top: portrait,
        // name, tagline, and bio. Below it the split begins — experience left,
        // the rail (certifications first) right — so certifications sits under
        // the header rather than beside it. One column below 60rem; the fit
        // dialog is a modal, out of the flow.
        (profile_header(resume))
        div class="resume-body" {
            div class="column column--main" {
                (experience(resume))
            }
            (right_rail(resume))
        }
        (fit_dialog())
    };

    shell(
        resume,
        ctx,
        Head {
            title: &title,
            description: &profile.summary,
            refresh: None,
            social: true,
        },
        body,
    )
}

fn profile_header(resume: &Resume) -> Markup {
    let profile = &resume.profile;

    html! {
        // The full-width header band. Identity (monogram, name, tagline,
        // location) on the left; the bio beside it on the right, so the band
        // fills the content width with two readable blocks rather than a
        // left-weighted header with an empty right. Stacks on a narrow screen.
        header class="resume-header" {
            div class="profile" {
                // The portrait, hex-clipped so the motif and the photo are one
                // decision rather than two. WebP primary with a PNG fallback;
                // a modern browser fetches only the few-KB WebP. Explicit
                // width/height reserve the box so the layout does not shift as
                // it loads. Real alt naming the person, not "photo"; the hex
                // clip is CSS on the img, so no decorative wrapper is needed.
                picture class="hex" {
                    source srcset=(crate::assets::PORTRAIT_WEBP.url()) type="image/webp";
                    img
                        class="hex__portrait"
                        src=(crate::assets::PORTRAIT_PNG.url())
                        width=(crate::assets::PORTRAIT_WIDTH)
                        height=(crate::assets::PORTRAIT_HEIGHT)
                        alt=(format!("{}, portrait", profile.full_name));
                }
                div {
                    h1 { (profile.full_name) }
                    p class="lede" { (profile.headline) }
                    // Location and contact email. Tenure is not repeated here:
                    // the bio states it in prose with context, so a derived
                    // "N years" beside the city would duplicate that claim and
                    // (computed from employed months, excluding the career
                    // break) disagree with it. One claim, one source.
                    p class="meta" {
                        (profile.location)
                        " · "
                        a href=(format!("mailto:{}", profile.email)) { (profile.email) }
                    }
                }
            }
            p class="profile-summary" { (profile.summary) }
        }
    }
}

fn experience(resume: &Resume) -> Markup {
    html! {
        section {
            h2 { "Experience" }
            ol class="timeline" {
                @for entry in resume.timeline() {
                    @match entry {
                        TimelineEntry::Role(role) => { (role_entry(role)) }
                        TimelineEntry::Break(gap) => { (break_entry(gap)) }
                    }
                }
            }
        }
    }
}

fn role_entry(role: &Role) -> Markup {
    html! {
        li class="entry" {
            h3 { (role.employer) }
            p class="meta" {
                (role.title)
                " · "
                (role.range_label())
                @if let Some(arrangement) = &role.arrangement {
                    " · " (arrangement)
                }
            }
            ul {
                @for highlight in &role.highlights {
                    li { (highlight) }
                }
            }
        }
    }
}

/// No employer, no title, no highlight list. A career break is a real period
/// on the timeline but it is not a job, and rendering it like one is how
/// parsers and screeners misread it.
fn break_entry(gap: &CareerBreak) -> Markup {
    html! {
        li class="entry entry--break" {
            h3 { (gap.reason) }
            p class="meta" { (gap.range_label()) }
            @if let Some(note) = &gap.note {
                p { (note) }
            }
        }
    }
}

fn skills(groups: &[SkillGroup]) -> Markup {
    html! {
        section {
            h2 { "Skills" }
            @for group in groups {
                div class="skill-group" {
                    h3 { (group.title) }
                    ul {
                        @for item in &group.items {
                            li { (item) }
                        }
                    }
                }
            }
        }
    }
}

/// The right rail, top to bottom: certifications, project links, the two
/// action buttons, then skills. The order is deliberate — certifications and
/// project links are short, so skills rises toward the fold beneath them.
/// Source order is the single-column reading order on a narrow screen.
fn right_rail(resume: &Resume) -> Markup {
    html! {
        div class="column rail" {
            @if !resume.certifications.is_empty() {
                section {
                    h2 { "Certifications" }
                    ul {
                        @for cert in &resume.certifications {
                            li { (cert.name) }
                        }
                    }
                }
            }
            @if !resume.projects.is_empty() {
                section {
                    h2 { "Projects" }
                    (project_links(&resume.projects))
                }
            }
            // The two actions, above skills. The compare button keeps the
            // id and href the fit modal upgrade binds to, so that wiring is
            // untouched; the lab button is a plain navigation link.
            nav class="rail-actions" aria-label="Explore" {
                a id="fit-trigger" href="/fit" class="button-link" {
                    "Compare a job description"
                }
                a href="/gen-ai" class="button-link" { "Explore the GenAI lab" }
            }
            (skills(&resume.skills))
        }
    }
}

/// Projects as links, not inline write-ups. A project links to its `url`, or
/// its `repo` if it has no url, and renders as a plain non-link item if it has
/// neither — a dead link is worse than an honest unlinked entry. Turning these
/// into internal project pages is a later phase.
fn project_links(projects: &[Project]) -> Markup {
    html! {
        ul class="rail-links" {
            @for project in projects {
                li {
                    @if let Some(href) = project.url.as_deref().or(project.repo.as_deref()) {
                        a href=(href) class="button-link" { (project.name) }
                    } @else {
                        div class="rail-links__plain" {
                            (project.name)
                            p class="meta" { (project.summary) }
                        }
                    }
                }
            }
        }
    }
}

/// Provenance in the page makes "is my change live" a lookup instead of a
/// guess.
fn footer(resume: &Resume, ctx: PageContext<'_>) -> Markup {
    html! {
        footer {
            p {
                "Built from " code { (ctx.commit) }
                " · schema v" (resume.schema_version)
                // Unobtrusive on purpose. A recruiter does not need it; the
                // engineer who wonders what this runs on will look here.
                " · " a href="/stack" { "stack" }
            }
        }
    }
}

/// The scope disclaimer, shown on every provenance view.
///
/// Not a footnote. A feature that implies "verified resume" while only
/// confirming that URLs resolve would be exactly the overclaiming this project
/// otherwise avoids, so the limit is stated where it cannot be missed.
fn provenance_scope() -> Markup {
    html! {
        p class="scope-note" {
            "This checks links, repositories, and published artifacts against "
            "their live sources. It "
            strong { "cannot" }
            " verify employment, job titles, or dates, because there is nothing "
            "public to check those against."
        }
    }
}

fn check_list(checks: &[Check]) -> Markup {
    html! {
        ul class="check-list" {
            @for check in checks {
                li {
                    span class="check-label" { (check.label) }
                    " "
                    code class="meta" { (check.source) }
                }
            }
        }
    }
}

/// The plan, shown before anything runs.
///
/// Showing the plan up front sets expectations about scope and makes the
/// limits concrete, rather than leaving them to a disclaimer nobody reads.
pub fn provenance_plan(resume: &Resume, ctx: PageContext<'_>, checks: &[Check]) -> Markup {
    let body = html! {
        header {
            h1 { "Provenance check" }
            p class="lede" { (checks.len()) " checks against live sources" }
        }

        (provenance_scope())

        section {
            h2 { "What will be checked" }
            (check_list(checks))
        }

        form method="post" action="/provenance" {
            button type="submit" { "Run the check" }
        }
    };

    shell(
        resume,
        ctx,
        Head {
            title: "Provenance check",
            description: "Verify the links and published artifacts on this site against their live sources.",
            refresh: None,
            social: false,
        },
        body,
    )
}

fn outcome_badge(outcome: &Outcome) -> Markup {
    let (class, label) = match outcome {
        Outcome::Passed { .. } => ("outcome--passed", "Passed"),
        Outcome::Unreachable { .. } => ("outcome--unreachable", "Unreachable"),
        Outcome::Mismatch { .. } => ("outcome--mismatch", "Mismatch"),
        Outcome::Skipped { .. } => ("outcome--skipped", "Skipped"),
    };

    html! {
        span class={ "outcome " (class) } { (label) }
    }
}

fn outcome_detail(outcome: &Outcome) -> Markup {
    html! {
        @match outcome {
            Outcome::Passed { detail } => { span class="meta" { (detail) } }
            Outcome::Skipped { reason } => { span class="meta" { (reason) } }
            // Phrased as an inability to reach, never as a failed claim. A
            // corporate site blocking datacenter addresses says nothing about
            // the employment.
            Outcome::Unreachable { reason } => {
                span class="meta" { "could not reach: " (reason) }
            }
            Outcome::Mismatch { expected, found } => {
                span class="meta" { "expected " code { (expected) } ", found " code { (found) } }
            }
        }
    }
}

/// Progress or results for one run.
pub fn provenance_run(
    resume: &Resume,
    ctx: PageContext<'_>,
    run: &CheckRun,
    checks: &[Check],
) -> Markup {
    let label_for = |id: &str| {
        checks
            .iter()
            .find(|check| check.id == id)
            .map_or("Unknown check", |check| check.label.as_str())
            .to_owned()
    };

    let body = html! {
        header {
            h1 { "Provenance check" }
            p class="lede" {
                @if run.status.is_terminal() {
                    (run.summary())
                } @else {
                    (run.status.label()) "…"
                }
            }
        }

        progress class="progress" value=(run.progress()) max="100" {
            (run.progress()) "%"
        }

        @if let Some(diagnostic) = &run.diagnostic {
            p class="scope-note" { "The run did not finish: " (diagnostic) }
        }

        @if !run.results.is_empty() {
            section {
                h2 { "Results" }
                ul class="check-list" {
                    @for result in &run.results {
                        li {
                            (outcome_badge(&result.outcome))
                            " "
                            span class="check-label" { (label_for(&result.check_id)) }
                            " "
                            (outcome_detail(&result.outcome))
                        }
                    }
                }
            }
        }

        @if run.status.is_terminal() {
            (provenance_scope())
            p { a href="/" { "Back to the resume" } }
        }
    };

    shell(
        resume,
        ctx,
        Head {
            title: "Provenance check",
            description: "Live verification of the links and artifacts on this site.",
            // Polls while running, stops once terminal. Without this the page
            // would need JavaScript to ever update.
            refresh: (!run.status.is_terminal()).then_some(3),
            social: false,
        },
        body,
    )
}

/// The drawing: hexagon nodes, curved connectors, tier labels.
///
/// Inline SVG with every coordinate computed from the content document. That
/// distinction matters: the reason the first attempt was ordered bands with no
/// edges was to avoid hand-positioned coordinates, but the result was boxes on
/// a page rather than a diagram. A layout function is not hand-positioning.
///
/// `aria-hidden`, and not a cop-out: the component list below is the same
/// information in a form a screen reader can actually navigate, including the
/// role text and the CloudFormation type that the drawing has no room for.
/// Describing a diagram twice, once badly, serves nobody.
fn architecture_svg(layout: &crate::architecture::Layout<'_>) -> Markup {
    use crate::architecture::Align;

    let view_box = format!("0 0 {:.0} {:.0}", layout.width, layout.height);

    html! {
        div class="diagram" {
            // Positions come from the generated rules in the stylesheet, keyed
            // by these ids. No `style` attributes anywhere: the policy hashes
            // the style block, which does not cover attributes, and opening
            // `'unsafe-hashes'` for a diagram would be a bad trade.
            div class="diagram__frame" {
                // Connectors only. SVG is the right tool for curves between
                // computed points and the wrong one for wrapping text, so the
                // nodes are HTML on top of this.
                svg
                    class="diagram__canvas"
                    viewBox=(view_box)
                    preserveAspectRatio="none"
                    aria-hidden="true"
                    focusable="false"
                {
                    g class="diagram__edges" {
                        @for connector in &layout.connectors {
                            path d=(connector.path()) {}
                        }
                    }
                }

                @for tier in &layout.tiers {
                    span
                        class="diagram__tier"
                        id=(format!("tier-{}", tier.id))
                        aria-hidden="true"
                    {
                        (tier.label)
                    }
                }

                @for placed in &layout.nodes {
                    @let describedby = format!("about-{}", placed.node.id);

                    // A button because it is focusable everywhere and a tap
                    // focuses it, which is what makes the detail reachable
                    // without a mouse. It performs no action; the popover is
                    // disclosure, shown on hover and focus alike.
                    button
                        type="button"
                        class={
                            "diagram__node"
                            @if placed.node.is_external() { " diagram__node--external" }
                            @if placed.placement.above { " diagram__node--flip" }
                            @match placed.placement.align {
                                Align::Start => " diagram__node--start",
                                Align::Centre => "",
                                Align::End => " diagram__node--end",
                            }
                        }
                        id=(format!("node-{}", placed.node.id))
                        aria-describedby=(describedby)
                    {
                        // Not `.hex`: that class carries a fixed inline-size
                        // for the portrait frame, which overrode this box and
                        // drew the shape larger than the button, putting the
                        // caption across the middle of it. The clip-path lives
                        // in `.diagram__shape` for exactly that reason.
                        span class="diagram__shape" aria-hidden="true" {}
                        span class="diagram__label" { (placed.node.label) }

                        span class="diagram__about" id=(describedby) role="tooltip" {
                            span class="diagram__about-role" { (placed.node.role) }
                            @match &placed.node.resource_type {
                                Some(kind) => span class="diagram__about-type" { (kind) },
                                None => span class="diagram__about-type" {
                                    "Outside this stack"
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One component in the list beneath the drawing.
///
/// This is the authoritative version: it carries the reasoning and the
/// CloudFormation type, neither of which fits in a hexagon.
fn architecture_node(node: &Node) -> Markup {
    html! {
        li class={ "node" @if node.is_external() { " node--external" } } {
            div class="hex node__badge" aria-hidden="true" {}
            div class="node__body" {
                // h2 Architecture, h3 Components, h4 tier, h5 here. Depth
                // rather than size: the visual weight comes from CSS.
                h5 { (node.label) }
                p { (node.role) }
                @match &node.resource_type {
                    // The type is the machine-checkable half of the claim, so
                    // it is worth showing rather than hiding in a data file.
                    Some(kind) => p class="meta" { code { (kind) } },
                    None => p class="meta" { "Outside this stack" },
                }
            }
        }
    }
}

/// The architecture diagram.
///
/// Semantic lists in ordered bands, not an image and not a client-side
/// renderer. A screen reader gets the structure for free, it themes from the
/// same custom properties as everything else, and it adds no bytes beyond the
/// markup. Flow runs from the first band to the last; there are no drawn edges
/// because routing them in CSS means hand-positioned coordinates, which is the
/// thing the old site did and this is meant to replace.
fn architecture_diagram(architecture: &Architecture) -> Markup {
    html! {
        section {
            h2 { "Architecture" }
            p class="lede" { (architecture.summary) }

            (architecture_svg(&architecture.layout()))

            p class="meta" {
                "Hover or focus any component for what it does and the "
                "CloudFormation type it deploys as."
            }

            // Collapsed, so the page leads with the drawing rather than a wall
            // of text, but the detail is still one click away and reads
            // linearly. Print styles reveal it regardless of state, because a
            // printed page has no hover and no disclosure triangle.
            details class="components" {
                summary { "Component reference" }
                ol class="tiers" {
                    @for tier in &architecture.tiers {
                        li class="tier" {
                            h4 class="tier__label" { (tier.label) }
                            ul class="nodes" {
                                @for node in &tier.nodes {
                                    (architecture_node(node))
                                }
                            }
                        }
                    }
                }
            }

            p class="scope-note" {
                "Each component names the CloudFormation type it deploys as, "
                "and the build fails if the diagram claims a type this stack "
                "does not contain. That catches a diagram describing something "
                "it does not run. It cannot prove the diagram is complete, so "
                "treat it as checked rather than exhaustive."
            }
        }
    }
}

/// `/stack`: what this project is built from.
///
/// Disclosure rather than verification, which is why it is a plain page with
/// no run, no polling, and no state machine behind it. Everything shown is
/// embedded at build time, so the page describes the commit in the footer.
pub fn stack_page(resume: &Resume, ctx: PageContext<'_>, architecture: &Architecture) -> Markup {
    let body = html! {
        header {
            h1 { "Stack" }
            p class="lede" {
                "What this site runs on and what it is built from, without a "
                "detour through a repository."
            }
        }

        (architecture_diagram(architecture))

        section {
            h2 { "Manifests" }
            p {
                "Verbatim, from the commit that built this page. A summarized "
                "dependency list is a claim; the file is the evidence."
            }

            @for manifest in MANIFESTS {
                article class="manifest" {
                    h3 { code { (manifest.name) } }
                    p { (manifest.note) }
                    pre class="manifest__body" {
                        code class={ "language-" (manifest.language) } { (manifest.body) }
                    }
                }
            }
        }
    };

    shell(
        resume,
        ctx,
        Head {
            title: "Stack — how this site is built",
            description: "The architecture and dependency manifests behind this site, \
                          embedded from the commit that built it.",
            refresh: None,
            social: false,
        },
        body,
    )
}

/// The honest limits of the fit analysis, shown on both fit views.
fn fit_scope() -> Markup {
    html! {
        p class="scope-note" {
            "This measures overlap with experience that is "
            strong { "written down here" }
            ", not suitability for the role. Requirements phrased in ways the "
            "taxonomy does not recognise are invisible to it, and a low score "
            "may mean the resume does not yet say something true."
        }
    }
}

/// The form fields, shared by the standalone page and the modal, so the two
/// cannot drift apart.
fn fit_fields(id: &str) -> Markup {
    html! {
        form method="post" action="/fit" class="fit-form" {
            label for=(id) { "Job description" }
            textarea
                id=(id)
                name="description"
                rows="14"
                required
                placeholder="Paste the posting here, or drag a highlighted selection onto this box." {}
            button type="submit" { "Analyse" }
        }
    }
}

/// The modal, plus the script that upgrades the link into it.
///
/// Progressive enhancement rather than a JavaScript-only feature: the trigger
/// is a real link to `/fit`, and the script only intercepts it when `<dialog>`
/// is actually supported. With scripting off, the link works.
fn fit_dialog() -> Markup {
    html! {
        dialog id="fit-dialog" class="fit-dialog" {
            form method="dialog" class="fit-dialog__dismiss" {
                button value="close" aria-label="Close" { "×" }
            }

            // The input side. Hidden rather than destroyed once a report
            // arrives, so "check another" restores it without a round trip
            // and without the script having to rebuild a form.
            div id="fit-dialog-panel" {
                h2 { "Job description fit" }
                (fit_scope())
                (fit_fields("fit-dialog-description"))
            }

            // Swapped into by the fragment endpoint. `aria-live` so the result
            // is announced rather than silently appearing, and `tabindex` so
            // focus can move here, which is what makes the swap navigable by
            // keyboard instead of leaving focus on a button that is now
            // hidden.
            div id="fit-dialog-result" hidden aria-live="polite" tabindex="-1" {}

            p id="fit-dialog-again" hidden {
                button type="button" { "Check another" }
            }
        }

        script { (maud::PreEscaped(ENHANCEMENT)) }
    }
}

/// The standalone page. The no-JavaScript baseline and the modal's fallback.
pub fn fit_form(resume: &Resume, ctx: PageContext<'_>) -> Markup {
    let body = html! {
        header {
            h1 { "Job description fit" }
            p class="lede" { "Paste a job description and see how it maps to real experience." }
        }

        (fit_scope())
        (fit_fields("description"))
    };

    shell(
        resume,
        ctx,
        Head {
            title: "Job description fit",
            description: "Paste a job description and see how it maps to documented experience.",
            refresh: None,
            social: false,
        },
        body,
    )
}

fn verdict_badge(verdict: Verdict) -> Markup {
    let (class, label) = match verdict {
        Verdict::Strong => ("outcome--passed", "Strong"),
        Verdict::Partial => ("outcome--unreachable", "Partial"),
        Verdict::Gap => ("outcome--mismatch", "Gap"),
    };

    html! { span class={ "outcome " (class) } { (label) } }
}

/// A heading at a chosen level.
///
/// maud needs a literal tag name, and the report renders both as its own
/// document and nested inside the dialog on the resume, where every heading
/// has to drop a level or the page outline becomes nonsense. One helper beats
/// branching at five call sites.
fn heading(level: u8, content: Markup) -> Markup {
    match level {
        1 => html! { h1 { (content) } },
        2 => html! { h2 { (content) } },
        3 => html! { h3 { (content) } },
        _ => html! { h4 { (content) } },
    }
}

/// Where a report is being rendered.
///
/// Changes the heading levels and the navigation, and nothing substantive.
/// Both surfaces call the same body function, so the analysis a visitor sees
/// in the modal cannot drift from the one on the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportDepth {
    /// The standalone page, which owns its document and therefore its `h1`.
    Page,
    /// Inside the dialog on the resume. The page's `h1` is the profile, so the
    /// report starts at `h2` and everything below shifts with it.
    Nested,
}

impl ReportDepth {
    fn title(self) -> u8 {
        match self {
            Self::Page => 1,
            Self::Nested => 2,
        }
    }

    fn section(self) -> u8 {
        self.title() + 1
    }

    fn dimension(self) -> u8 {
        self.section() + 1
    }
}

fn fit_dimension(
    resume: &Resume,
    report: &FitReport,
    dimension: Dimension,
    level: u8,
) -> Option<Markup> {
    let rows: Vec<(&Requirement, &Assessment)> = report
        .requirements
        .iter()
        .filter(|requirement| requirement.dimension == dimension)
        .filter_map(|requirement| {
            report
                .assessments
                .iter()
                .find(|assessment| assessment.requirement_id == requirement.id)
                .map(|assessment| (requirement, assessment))
        })
        .collect();

    if rows.is_empty() {
        return None;
    }

    Some(html! {
        (heading(level, html! { (dimension.label()) }))
        ul class="check-list" {
            @for (requirement, assessment) in rows {
                li {
                    (verdict_badge(assessment.verdict))
                    " "
                    span class="check-label" {
                        (requirement.text)
                        @if requirement.kind == RequirementKind::MustHave {
                            " " span class="meta" { "(required)" }
                        }
                    }
                    " "
                    span class="meta" { (assessment.rationale) }

                    // The cited resume text, so a claim can be checked rather
                    // than taken on trust.
                    @if !assessment.citations.is_empty() {
                        ul class="citations" {
                            @for citation in &assessment.citations {
                                @if let Some(text) = citation.resolve(resume) {
                                    li { (text) }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

/// The report itself, with no document around it.
///
/// Shared by the standalone page and the dialog fragment so the two cannot
/// drift. Everything that differs between them is heading depth and
/// navigation, both of which come from `depth`.
fn fit_report_body(resume: &Resume, report: &FitReport, depth: ReportDepth) -> Markup {
    let emphasis = report.emphasis(resume);
    let gaps: Vec<&Assessment> = report.gaps().collect();

    html! {
        header {
            (heading(depth.title(), html! { "Job description fit" }))
            p class="lede" { (report.tier().label()) " · " (report.score()) " / 100" }
            p class="meta" {
                (report.requirements.len()) " requirements recognised · "
                (report.provenance.method) " rev " (report.provenance.revision)
            }

            // Stated next to the score rather than in a footnote. A score over
            // partial coverage looks identical to a score over full coverage,
            // so the reader cannot calibrate without this, and quietly
            // dropping requirements is the failure this feature would be
            // judged on.
            @if report.unmatched_terms > 0 {
                p class="scope-note" {
                    "Coverage: up to " (report.unmatched_terms)
                    @if report.unmatched_terms == 1 { " requirement " } @else { " requirements " }
                    "in the posting matched nothing in the taxonomy, so the "
                    "score does not account for them."
                }
            }

            // A partial report is better than no report, but only if it says
            // which part is missing. Omitting this would make a degraded run
            // look identical to a clean one.
            @if !report.incomplete_dimensions.is_empty() {
                p class="scope-note" {
                    "This report is incomplete: "
                    @for (index, dimension) in report.incomplete_dimensions.iter().enumerate() {
                        @if index > 0 { ", " }
                        (dimension.label())
                    }
                    @if report.incomplete_dimensions.len() == 1 {
                        " was not assessed."
                    } @else {
                        " were not assessed."
                    }
                    " The score covers the rest."
                }
            }
        }

        // First, because it is the only section that answers a question the
        // reader cannot answer themselves.
        @if !emphasis.is_empty() {
            section {
                (heading(depth.section(), html! { "Lead with these" }))
                ul {
                    @for point in &emphasis {
                        li { (point) }
                    }
                }
            }
        }

        section {
            (heading(depth.section(), html! { "Requirement mapping" }))
            @for dimension in Dimension::ALL {
                @if let Some(markup) =
                    fit_dimension(resume, report, dimension, depth.dimension())
                {
                    (markup)
                }
            }
        }

        // Stated plainly rather than buried. A report that claims everything
        // matches is useless to a reader.
        section {
            (heading(depth.section(), html! { "Gaps" }))
            @if gaps.is_empty() {
                p { "Nothing in the recognised requirements is unaddressed." }
            } @else {
                ul class="check-list" {
                    @for assessment in gaps {
                        @let requirement = report
                            .requirements
                            .iter()
                            .find(|r| r.id == assessment.requirement_id);
                        li {
                            span class="check-label" {
                                @if let Some(requirement) = requirement {
                                    (requirement.text)
                                } @else {
                                    (assessment.requirement_id)
                                }
                            }
                            " "
                            span class="meta" { (assessment.rationale) }
                        }
                    }
                }
            }
        }

        (fit_scope())

        // Only on the standalone page. Inside the dialog the visitor never
        // left the resume, so "back to the resume" would be nonsense and the
        // dialog's own controls handle both of these.
        @if depth == ReportDepth::Page {
            p {
                a href="/fit" { "Analyse another" }
                " · "
                a href="/" { "Back to the resume" }
            }
        }
    }
}

/// The result as a standalone page. The no-JavaScript path lands here.
pub fn fit_report(resume: &Resume, ctx: PageContext<'_>, report: &FitReport) -> Markup {
    shell(
        resume,
        ctx,
        Head {
            title: "Job description fit",
            description: "How this experience maps to a job description.",
            refresh: None,
            social: false,
        },
        fit_report_body(resume, report, ReportDepth::Page),
    )
}

/// The result as a fragment, for swapping into the dialog on the resume.
///
/// A separate function rather than content negotiation on `/fit`: a fragment
/// endpoint returns fragments and a page endpoint returns documents, and
/// deciding which deep inside one handler is how that distinction rots.
pub fn fit_report_fragment(resume: &Resume, report: &FitReport) -> Markup {
    fit_report_body(resume, report, ReportDepth::Nested)
}

// Kept last in the file: clippy's `items_after_test_module` fires otherwise,
// and a test module buried mid-file is harder to find anyway.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::load;

    const CTX: PageContext<'static> = PageContext {
        commit: "abc1234",
        as_of: ImpreciseDate::YearMonth(2026, 10),
        site_url: None,
        provenance_enabled: true,
    };

    fn rendered() -> String {
        page(&load().unwrap(), CTX).into_string()
    }

    #[test]
    fn renders_a_complete_document() {
        let html = rendered();
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<html lang=\"en\">"));
        assert!(html.contains("National Insurance Producer Registry"));
        assert!(html.contains("abc1234"));
    }

    #[test]
    fn career_break_renders_without_employer_framing() {
        let html = rendered();
        assert!(html.contains("entry entry--break"));
        assert!(html.contains("Full-time family caregiving"));
    }

    /// Builds a report through the real extraction and matching path, so the
    /// rendered output cannot drift from what the pipeline actually produces.
    fn report_for(description: &str) -> (Resume, FitReport) {
        let resume = load().unwrap();
        let taxonomy = crate::taxonomy::Taxonomy::load().unwrap();
        let job = crate::fit::JobDescription::new(description).unwrap();

        let extraction = crate::taxonomy::extract(&taxonomy, &job);
        let requirements = extraction.requirements.clone();

        let branches = Dimension::ALL
            .iter()
            .map(|dimension| crate::match_dimension(*dimension, &requirements, &resume, CTX.as_of))
            .collect();

        let report = crate::aggregate(
            requirements,
            branches,
            crate::fit::Provenance {
                method: crate::fit::DETERMINISTIC_METHOD.to_owned(),
                revision: taxonomy.revision.clone(),
                generated_at: "2026-10-25T00:00:00Z".to_owned(),
            },
            extraction.unmatched_terms,
        );

        (resume, report)
    }

    #[test]
    fn the_policy_authorizes_exactly_what_the_document_inlines() {
        let policy = content_security_policy();

        // The hashes are computed from the same strings the template inlines,
        // so this proves the two cannot drift. A wrong script hash means the
        // modal silently stops opening; a wrong style hash means an unstyled
        // page.
        //
        // Hashing `INLINE_STYLES` rather than `STYLES`: the inlined block is
        // the authored CSS plus the diagram's generated geometry, and the
        // policy has to cover what is actually in the document.
        assert!(policy.contains(&csp_hash(&INLINE_STYLES)));
        assert!(policy.contains(&csp_hash(ENHANCEMENT)));

        // Closed by default, and no blanket escape hatch anywhere.
        assert!(policy.starts_with("default-src 'none'"));
        assert!(!policy.contains("unsafe-inline"));
        assert!(!policy.contains("unsafe-eval"));
        assert!(!policy.contains('*'));
    }

    #[test]
    fn hashes_are_well_formed_and_content_sensitive() {
        let hash = csp_hash("alert(1)");

        assert!(hash.starts_with("'sha256-") && hash.ends_with('\''));
        // Base64 of a 32 byte digest is 44 characters including padding.
        assert_eq!(hash.len(), "'sha256-".len() + 44 + 1);
        assert_ne!(csp_hash("alert(1)"), csp_hash("alert(2)"));
    }

    #[test]
    fn every_page_carries_the_policy_before_what_it_governs() {
        let resume = load().unwrap();
        let architecture = crate::architecture::load().unwrap();

        let plan = crate::provenance::plan(&resume, CTX.commit);

        let pages = [
            rendered(),
            fit_form(&resume, CTX).into_string(),
            stack_page(&resume, CTX, &architecture).into_string(),
            provenance_plan(&resume, CTX, &plan).into_string(),
        ];

        for html in &pages {
            let policy_at = html
                .find("Content-Security-Policy")
                .expect("policy is present");
            let style_at = html.find("<style>").expect("styles are inlined");
            assert!(
                policy_at < style_at,
                "a policy parsed after the content it authorizes does nothing"
            );
        }
    }

    #[test]
    fn nothing_rendered_needs_an_escape_hatch_in_the_policy() {
        let resume = load().unwrap();
        let architecture = crate::architecture::load().unwrap();
        let plan = crate::provenance::plan(&resume, CTX.commit);

        let pages = [
            rendered(),
            fit_form(&resume, CTX).into_string(),
            stack_page(&resume, CTX, &architecture).into_string(),
            provenance_plan(&resume, CTX, &plan).into_string(),
        ];

        // A block hash covers a `<style>` or `<script>` element, not a
        // `style=` attribute or an `onclick=`. Those would need
        // `'unsafe-hashes'` or `'unsafe-inline'`, and adding either to make one
        // ornament work would quietly undo the policy. Keeping them out is the
        // condition that lets the policy stay strict.
        for html in &pages {
            for banned in [
                "style=\"",
                "onclick=",
                "onsubmit=",
                "oninput=",
                "javascript:",
            ] {
                assert!(
                    !html.contains(banned),
                    "{banned} is blocked by the policy and would need it weakened"
                );
            }
        }
    }

    #[test]
    fn stack_page_shows_the_diagram_and_the_manifests() {
        let resume = load().unwrap();
        let architecture = crate::architecture::load().unwrap();
        let html = stack_page(&resume, CTX, &architecture).into_string();

        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains(&architecture.summary));

        // Every component appears, with the CloudFormation type that makes the
        // claim checkable.
        for node in architecture.nodes() {
            assert!(html.contains(&node.label), "{} is missing", node.id);
            if let Some(kind) = &node.resource_type {
                assert!(html.contains(kind.as_str()), "{kind} is not shown");
            }
        }

        // Manifests verbatim, not summarized.
        assert!(html.contains("[workspace.dependencies]"));
        assert!(html.contains("aws-cdk-lib"));
    }

    #[test]
    fn the_architecture_renders_as_a_drawing_not_a_list_of_boxes() {
        let resume = load().unwrap();
        let architecture = crate::architecture::load().unwrap();
        let layout = architecture.layout();
        let html = stack_page(&resume, CTX, &architecture).into_string();

        // A real drawing: a canvas, a hexagon per component, and a connector
        // per edge. The first version of this page had nodes and no edges,
        // which is a list with borders rather than a diagram.
        assert!(html.contains("<svg"));
        assert_eq!(html.matches("<path").count(), layout.connectors.len());
        assert!(layout.connectors.len() > 10, "a diagram needs edges");

        // A node per component, each focusable so its detail is reachable
        // without a pointer, and each pointing at its own description.
        assert_eq!(
            html.matches(r#"class="diagram__node"#).count(),
            layout.nodes.len()
        );
        for placed in &layout.nodes {
            assert!(html.contains(&format!(r#"id="node-{}""#, placed.node.id)));
            assert!(html.contains(&format!(r#"aria-describedby="about-{}""#, placed.node.id)));
        }

        // Scales rather than being pinned to one width.
        assert!(html.contains("viewBox="));
    }

    #[test]
    fn every_component_detail_is_reachable_without_hover() {
        let resume = load().unwrap();
        let architecture = crate::architecture::load().unwrap();
        let html = stack_page(&resume, CTX, &architecture).into_string();

        // Hover-only disclosure is unreachable by keyboard, by touch, and by
        // a screen reader. The text is in the document and associated with a
        // focusable control, so hover is an accelerator rather than the only
        // way in.
        for node in architecture.nodes() {
            assert!(
                html.contains(&node.role),
                "{} explains itself only on hover",
                node.id
            );
        }

        // And it still reads linearly for print and for anyone who would
        // rather scan a list than poke at a drawing.
        assert!(html.contains("Component reference"));
    }

    #[test]
    fn the_diagram_positions_itself_without_inline_styles() {
        let resume = load().unwrap();
        let architecture = crate::architecture::load().unwrap();
        let html = stack_page(&resume, CTX, &architecture).into_string();

        // The policy hashes the style block, which does not cover `style=`
        // attributes. Positioning with them would have needed
        // `'unsafe-hashes'`, so the geometry is generated into the stylesheet
        // instead and covered by the same hash.
        assert!(!html.contains("style=\""));
        assert!(content_security_policy().contains(&csp_hash(&INLINE_STYLES)));

        // The generated rules are really there, keyed to the component ids.
        let css = architecture.layout_css();
        assert!(css.contains("#node-site-handler{left:"));
        assert!(css.contains(".diagram__frame{aspect-ratio:"));
        assert!(INLINE_STYLES.contains(&css));
    }

    #[test]
    fn the_drawing_carries_no_hard_coded_colour() {
        let resume = load().unwrap();
        let architecture = crate::architecture::load().unwrap();
        let svg = stack_page(&resume, CTX, &architecture).into_string();

        // Fills and strokes come from the stylesheet, so the drawing themes
        // with the rest of the site and prints light without a second code
        // path. A literal hex here would be invisible to both.
        assert!(!svg.contains("fill=\"#"));
        assert!(!svg.contains("stroke=\"#"));
    }

    #[test]
    fn stack_page_states_what_the_check_does_not_prove() {
        let resume = load().unwrap();
        let architecture = crate::architecture::load().unwrap();
        let html = stack_page(&resume, CTX, &architecture).into_string();

        // The diagram is verified one way only. Claiming completeness would be
        // the overclaiming this project avoids everywhere else.
        assert!(html.contains("cannot prove the diagram is complete"));
    }

    #[test]
    fn stack_page_never_leaks_an_account_identifier() {
        let resume = load().unwrap();
        let architecture = crate::architecture::load().unwrap();
        let html = stack_page(&resume, CTX, &architecture).into_string();

        // This is why cdk.json is not published. If a manifest ever starts
        // carrying the hosted zone id, it fails here as well as in the
        // architecture module.
        assert!(!html.contains("hostedZoneId"));
        assert!(!html.contains("Z05927452DEMZDEUMM8PW"));
    }

    #[test]
    fn decorative_diagram_badges_are_hidden_from_assistive_technology() {
        let resume = load().unwrap();
        let architecture = crate::architecture::load().unwrap();
        let html = stack_page(&resume, CTX, &architecture).into_string();

        // The hexagons carry no information; the label and role next to them
        // do. An unlabelled shape announced to a screen reader is noise.
        assert!(html.contains(r#"<div class="hex node__badge" aria-hidden="true">"#));
    }

    #[test]
    fn the_stack_page_is_reachable_from_the_resume() {
        assert!(
            rendered().contains(r#"<a href="/stack">"#),
            "an unlinked page may as well not exist"
        );
    }

    #[test]
    fn unrecognised_requirements_are_disclosed_with_a_count() {
        let (resume, report) = report_for(
            "Requirements: TypeScript, commercial driving licence, \
             underwater welding certification.",
        );

        assert_eq!(report.unmatched_terms, 2);

        let html = fit_report(&resume, CTX, &report).into_string();
        assert!(
            html.contains("Coverage: up to 2 requirements"),
            "a score computed over partial coverage must say so"
        );
    }

    #[test]
    fn the_fragment_is_the_report_without_a_document_around_it() {
        let (resume, report) = report_for("Requirements: TypeScript, React, accessibility.");
        let fragment = fit_report_fragment(&resume, &report).into_string();

        // No document furniture: this gets inserted into an open dialog.
        assert!(!fragment.contains("<!DOCTYPE html>"));
        assert!(!fragment.contains("<html"));
        assert!(!fragment.contains("<style>"));
        assert!(!fragment.contains("<footer"));

        // The substance is all there.
        assert!(fragment.contains(report.tier().label()));
        assert!(fragment.contains("Requirement mapping"));
        assert!(fragment.contains("Gaps"));
    }

    #[test]
    fn the_fragment_and_the_page_report_the_same_analysis() {
        let (resume, report) = report_for("Requirements: TypeScript, React, accessibility.");

        let page_html = fit_report(&resume, CTX, &report).into_string();
        let fragment = fit_report_fragment(&resume, &report).into_string();

        // Both surfaces call one body function, so the only differences are
        // heading depth and navigation. A visitor must not be able to get a
        // different answer by opening the modal instead of the page.
        for requirement in &report.requirements {
            assert_eq!(
                page_html.contains(&requirement.text),
                fragment.contains(&requirement.text),
                "{} appears on one surface but not the other",
                requirement.id
            );
        }
        assert!(page_html.contains(report.tier().label()));
        assert!(fragment.contains(report.tier().label()));
    }

    #[test]
    fn the_fragment_starts_a_heading_level_below_the_page() {
        let (resume, report) = report_for("Requirements: TypeScript, accessibility.");

        let page_html = fit_report(&resume, CTX, &report).into_string();
        let fragment = fit_report_fragment(&resume, &report).into_string();

        // The page owns its document, so the report titles it with h1. The
        // fragment lands inside a page whose h1 is the profile, so everything
        // shifts down one or the outline becomes nonsense.
        assert!(page_html.contains("<h1>Job description fit</h1>"));
        assert!(fragment.contains("<h2>Job description fit</h2>"));
        assert!(!fragment.contains("<h1"));

        assert!(page_html.contains("<h2>Requirement mapping</h2>"));
        assert!(fragment.contains("<h3>Requirement mapping</h3>"));
    }

    #[test]
    fn only_the_page_offers_navigation_away() {
        let (resume, report) = report_for("Requirements: TypeScript.");

        assert!(
            fit_report(&resume, CTX, &report)
                .into_string()
                .contains(r#"<a href="/">"#)
        );
        // Inside the dialog the visitor never left the resume, so a link back
        // to it would be nonsense.
        assert!(
            !fit_report_fragment(&resume, &report)
                .into_string()
                .contains(r#"<a href="/">"#)
        );
    }

    #[test]
    fn the_dialog_has_somewhere_to_put_a_report_and_stays_usable_without_script() {
        let html = rendered();

        // Containers the swap targets.
        assert!(html.contains(r#"id="fit-dialog-panel""#));
        assert!(html.contains(r#"id="fit-dialog-result""#));
        assert!(html.contains(r#"id="fit-dialog-again""#));

        // Announced and focusable, so the swap is navigable rather than a
        // silent change that leaves focus on a now-hidden button.
        assert!(html.contains(r#"aria-live="polite""#));

        // The no-script path is untouched: the form in the dialog still posts
        // to the page endpoint on its own.
        assert!(html.contains(r#"action="/fit""#));
        assert!(html.contains(r#"method="post""#));
    }

    #[test]
    fn the_policy_allows_the_one_request_the_modal_makes() {
        let policy = content_security_policy();

        // The fetch to the fragment endpoint needs this, and same-origin only
        // means the pasted description cannot be sent anywhere else.
        assert!(policy.contains("connect-src 'self'"));
        assert!(!policy.contains("connect-src *"));
    }

    #[test]
    fn a_tenure_requirement_renders_the_employment_it_cites() {
        let (resume, report) =
            report_for("Senior Engineer. Requirements: 8+ years of experience, TypeScript.");

        report
            .validate(&resume)
            .expect("a tenure claim must cite something that resolves");

        let html = fit_report(&resume, CTX, &report).into_string();

        // The rationale states the arithmetic, and the citations name the
        // employments it sums, so a reader can check the claim instead of
        // taking the number on trust.
        assert!(html.contains("years of employment on record"));
        assert!(
            resume
                .roles
                .iter()
                .all(|role| html.contains(&role.employer)),
            "every employment the total rests on should be cited"
        );
    }

    #[test]
    fn a_failed_branch_yields_a_partial_report_that_admits_it() {
        let resume = load().unwrap();
        let taxonomy = crate::taxonomy::Taxonomy::load().unwrap();
        let job =
            crate::fit::JobDescription::new("Requirements: TypeScript, accessibility, mentoring.")
                .unwrap();

        let requirements = crate::taxonomy::extract(&taxonomy, &job).requirements;
        assert!(
            requirements
                .iter()
                .any(|r| r.dimension == Dimension::Seniority),
            "the fixture needs a seniority requirement for this to mean anything"
        );

        // Every branch but seniority, which is what the Parallel state hands
        // back when that branch is caught and returns an empty result.
        let branches = Dimension::ALL
            .iter()
            .map(|dimension| {
                if *dimension == Dimension::Seniority {
                    Vec::new()
                } else {
                    crate::match_dimension(*dimension, &requirements, &resume, CTX.as_of)
                }
            })
            .collect();

        let report = crate::aggregate(
            requirements,
            branches,
            crate::fit::Provenance {
                method: crate::fit::DETERMINISTIC_METHOD.to_owned(),
                revision: taxonomy.revision.clone(),
                generated_at: "2026-10-25T00:00:00Z".to_owned(),
            },
            0,
        );

        assert_eq!(report.incomplete_dimensions, vec![Dimension::Seniority]);
        assert!(
            !report.assessments.is_empty(),
            "a partial report, not no report"
        );

        let html = fit_report(&resume, CTX, &report).into_string();
        assert!(html.contains("This report is incomplete"));
        assert!(html.contains(Dimension::Seniority.label()));
    }

    #[test]
    fn a_complete_report_makes_no_incompleteness_claim() {
        let (resume, report) = report_for("Requirements: TypeScript, React, accessibility.");

        assert!(report.incomplete_dimensions.is_empty());
        assert!(
            !fit_report(&resume, CTX, &report)
                .into_string()
                .contains("This report is incomplete")
        );
    }

    #[test]
    fn full_coverage_makes_no_claim_about_missed_requirements() {
        let (resume, report) = report_for("Requirements: TypeScript, React, accessibility.");

        assert_eq!(report.unmatched_terms, 0);
        assert!(
            !fit_report(&resume, CTX, &report)
                .into_string()
                .contains("Coverage: up to")
        );
    }

    #[test]
    fn the_portrait_is_content_not_decoration() {
        let html = rendered();
        // The hex now frames the portrait, which is the person, so it carries
        // meaningful alt and is not hidden from assistive technology. (The old
        // monogram placeholder was decorative and aria-hidden; the real photo
        // is not.)
        assert!(
            html.contains("alt=") && html.contains(", portrait"),
            "the portrait must carry descriptive alt text naming the person"
        );
        // The portrait image is served and referenced.
        assert!(
            html.contains(".webp") && html.contains("portrait"),
            "the portrait sources must be present"
        );
        // The bracket motif is CSS generated content, never markup.
        assert!(!html.contains("&lt; Experience &gt;"));
    }

    #[test]
    fn heading_outline_is_sensible() {
        let html = rendered();
        assert_eq!(html.matches("<h1>").count(), 1, "exactly one h1");
        assert!(html.matches("<h2>").count() >= 3);
    }

    #[test]
    fn styles_are_inlined_and_include_print_rules() {
        let html = rendered();
        assert!(html.contains("--accent-warm"));
        assert!(html.contains("@media print"));
    }

    #[test]
    fn fit_trigger_is_a_real_link_with_a_dialog_behind_it() {
        let html = rendered();

        // Without scripting the trigger must still go somewhere useful.
        assert!(html.contains(r#"id="fit-trigger" href="/fit""#));
        assert!(html.contains(r#"<dialog id="fit-dialog""#));
        assert!(html.contains("showModal"));
    }

    #[test]
    fn fit_form_fields_match_between_page_and_modal() {
        let resume = load().unwrap();
        let page_html = fit_form(&resume, CTX).into_string();
        let home_html = rendered();

        // Same field name, so one endpoint serves both.
        assert!(page_html.contains(r#"name="description""#));
        assert!(home_html.contains(r#"name="description""#));
        assert!(page_html.contains(r#"action="/fit""#));
        assert!(home_html.contains(r#"action="/fit""#));
    }

    #[test]
    fn provenance_plan_states_its_limits_and_lists_checks() {
        let resume = load().unwrap();
        let checks = crate::provenance::plan(&resume, "abc1234");
        let html = provenance_plan(&resume, CTX, &checks).into_string();

        assert!(html.contains("cannot"));
        assert!(html.contains("verify employment"));
        assert!(html.contains("Run the check"));
        assert!(html.contains(&checks[0].label));
    }

    #[test]
    fn running_page_polls_and_terminal_page_does_not() {
        use crate::provenance::{CheckRun, RunStatus};

        let resume = load().unwrap();
        let checks = crate::provenance::plan(&resume, "abc1234");

        let mut run = CheckRun::queued("run1", checks.len());
        run.status = RunStatus::Running;
        let running = provenance_run(&resume, CTX, &run, &checks).into_string();
        assert!(running.contains("http-equiv=\"refresh\""));

        run.status = RunStatus::Complete;
        let done = provenance_run(&resume, CTX, &run, &checks).into_string();
        assert!(!done.contains("http-equiv=\"refresh\""));
        assert!(done.contains("checks passed"));
    }

    #[test]
    fn unreachable_is_never_phrased_as_a_failed_claim() {
        use crate::provenance::{CheckResult, CheckRun, Outcome, RunStatus};

        let resume = load().unwrap();
        let checks = crate::provenance::plan(&resume, "abc1234");

        let mut run = CheckRun::queued("run1", checks.len());
        run.status = RunStatus::Complete;
        run.results = vec![CheckResult {
            check_id: checks[0].id.clone(),
            outcome: Outcome::Unreachable {
                reason: "timed out".into(),
            },
            duration_ms: 5000,
        }];

        let html = provenance_run(&resume, CTX, &run, &checks).into_string();
        assert!(html.contains("could not reach"));
        assert!(!html.contains("Failed"));
    }

    #[test]
    fn the_resume_page_never_offers_a_provenance_trigger() {
        // The resume page dropped the provenance trigger in the phase 9 reskin.
        // The feature and its route stay, but the resume no longer surfaces a
        // button for it, regardless of whether the run store is available.
        let resume = load().unwrap();
        for provenance_enabled in [true, false] {
            let ctx = PageContext {
                provenance_enabled,
                ..CTX
            };
            let html = page(&resume, ctx).into_string();
            assert!(
                !html.contains(r#"action="/provenance""#),
                "the resume page must not offer a provenance trigger (enabled={provenance_enabled})"
            );
        }
        // The fit feature stays, as the compare-a-job-description action.
        let html = page(&resume, CTX).into_string();
        assert!(html.contains(r#"href="/fit""#));
    }

    #[test]
    fn the_resume_links_to_the_gen_ai_lab() {
        // An unlinked lab may as well not exist — the same rationale as the
        // /stack link test.
        assert!(
            rendered().contains(r#"href="/gen-ai""#),
            "the resume must offer a way into the GenAI lab"
        );
    }

    #[test]
    fn the_right_rail_orders_certifications_then_projects_then_skills() {
        // The rail's required top-to-bottom order, asserted on the byte
        // positions of the section headings in source order (which is also the
        // single-column reading order).
        // Certifications before skills always; the projects section only
        // appears when the document has projects, so its position is checked
        // only when present.
        let html = rendered();
        let certs = html
            .find(">Certifications<")
            .expect("certifications heading");
        let skills = html.find(">Skills<").expect("skills heading");
        assert!(
            certs < skills,
            "rail order must put certifications above skills"
        );
        if let Some(projects) = html.find(">Projects<") {
            assert!(
                certs < projects && projects < skills,
                "when present, projects sit between certifications and skills"
            );
        }
    }

    #[test]
    fn omits_canonical_without_an_origin() {
        let html = rendered();
        assert!(html.contains("og:title"));
        assert!(!html.contains("rel=\"canonical\""));
        assert!(!html.contains("og:url"));
    }

    #[test]
    fn emits_canonical_with_an_origin() {
        let ctx = PageContext {
            site_url: Some("https://example.com"),
            ..CTX
        };
        let html = page(&load().unwrap(), ctx).into_string();
        assert!(html.contains("rel=\"canonical\" href=\"https://example.com\""));
    }

    #[test]
    fn sitemap_requires_an_absolute_origin() {
        assert!(sitemap_xml(None).is_none());

        let xml = sitemap_xml(Some("https://example.com")).unwrap();
        assert!(xml.contains("<loc>https://example.com/</loc>"));
    }

    #[test]
    fn robots_points_at_the_sitemap_when_known() {
        assert!(!robots_txt(None).contains("Sitemap:"));
        assert!(
            robots_txt(Some("https://example.com"))
                .contains("Sitemap: https://example.com/sitemap.xml")
        );
    }
}
