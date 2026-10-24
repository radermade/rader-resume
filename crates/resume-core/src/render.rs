//! Markup, rendered from typed data.
//!
//! These functions take content and return `Markup`. No AWS types, no HTTP
//! types, no environment reads. That is what lets the same functions run in a
//! request handler now and in a pipeline render task later, which is the whole
//! point of open question 7 resolving to static-first.

use maud::{DOCTYPE, Markup, html};

use crate::{CareerBreak, ImpreciseDate, Project, Resume, Role, SkillGroup, TimelineEntry};

/// Styles are inlined. One request, well inside the page weight budget, and no
/// render-blocking stylesheet fetch.
const STYLES: &str = include_str!("styles.css");

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
}

/// Initials for the monogram, until there is a portrait to put there.
///
/// Preferred name plus surname, so "William \"Matt\" Rader" gives "MR" rather
/// than the "WM" a naive first-two-words split would produce.
fn monogram(preferred: &str, full_name: &str) -> String {
    let first = preferred.chars().find(char::is_ascii_alphabetic);
    let last = full_name
        .split_whitespace()
        .next_back()
        .and_then(|word| word.chars().find(char::is_ascii_alphabetic));

    [first, last]
        .into_iter()
        .flatten()
        .collect::<String>()
        .to_uppercase()
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

pub fn page(resume: &Resume, ctx: PageContext<'_>) -> Markup {
    let profile = &resume.profile;

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="description" content=(profile.summary);
                title { (profile.full_name) " — " (profile.headline) }
                (social_meta(resume, ctx))
                style { (maud::PreEscaped(STYLES)) }
            }
            body {
                main {
                    (profile_header(resume, ctx))
                    (experience(resume))
                    (skills(&resume.skills))
                    @if !resume.projects.is_empty() {
                        (projects(&resume.projects))
                    }
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
                }
                (footer(resume, ctx))
            }
        }
    }
}

fn profile_header(resume: &Resume, ctx: PageContext<'_>) -> Markup {
    let profile = &resume.profile;

    html! {
        header class="profile" {
            // Decorative until a portrait exists. Hidden from assistive
            // technology because the name is already in the heading.
            div class="hex" aria-hidden="true" {
                (monogram(&profile.preferred_name, &profile.full_name))
            }
            div {
                h1 { (profile.full_name) }
                p class="lede" { (profile.headline) }
                p class="meta" {
                    (profile.location)
                    " · "
                    (resume.experience_years(ctx.as_of)) " years"
                }
            }
        }
        p { (profile.summary) }
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

fn projects(projects: &[Project]) -> Markup {
    html! {
        section {
            h2 { "Projects" }
            @for project in projects {
                div class="entry" {
                    h3 { (project.name) }
                    p class="meta" { (project.status) }
                    p { (project.summary) }
                    @if !project.highlights.is_empty() {
                        ul {
                            @for highlight in &project.highlights {
                                li { (highlight) }
                            }
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
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load;

    const CTX: PageContext<'static> = PageContext {
        commit: "abc1234",
        as_of: ImpreciseDate::YearMonth(2026, 10),
        site_url: None,
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

    #[test]
    fn decoration_is_hidden_from_assistive_technology() {
        let html = rendered();
        assert!(html.contains("aria-hidden=\"true\""));
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
    fn monogram_uses_preferred_name_and_surname() {
        assert_eq!(monogram("Matt", "William \"Matt\" Rader"), "MR");
        assert_eq!(monogram("Ada", "Ada Lovelace"), "AL");
    }

    #[test]
    fn styles_are_inlined_and_include_print_rules() {
        let html = rendered();
        assert!(html.contains("--accent-warm"));
        assert!(html.contains("@media print"));
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
