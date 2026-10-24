//! Content validation.
//!
//! Parsing proves the document matches the schema. Validation proves it is fit
//! to publish, which is a higher bar: a resume PDF reaching a recruiter with
//! `[Current Title]` in it is worse than any outage this project could have.

use crate::{ImpreciseDate, Resume, SCHEMA_VERSION};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub path: String,
    pub detail: String,
}

impl ValidationIssue {
    fn new(path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            detail: detail.into(),
        }
    }
}

/// Authored content never contains square brackets. The bracket motif lives in
/// templates and CSS, not in the record, so a bracket here is an unfilled
/// placeholder.
fn is_placeholder(text: &str) -> bool {
    text.contains('[') && text.contains(']')
}

/// Returns an issue if a field is unfit to publish.
///
/// A function returning `Option` rather than a closure that pushes: a closure
/// capturing the issue list holds a mutable borrow for its whole lifetime,
/// which blocks every other push in the same scope.
fn issue_for(path: String, value: &str) -> Option<ValidationIssue> {
    if value.trim().is_empty() {
        Some(ValidationIssue::new(path, "is empty"))
    } else if is_placeholder(value) {
        Some(ValidationIssue::new(path, "unresolved placeholder"))
    } else {
        None
    }
}

/// Extracts a stated year count from prose, e.g. `"15 years building"` -> 15.
fn stated_years(summary: &str) -> Option<u32> {
    let tokens: Vec<&str> = summary.split_whitespace().collect();
    tokens
        .windows(2)
        .find_map(|pair| pair[1].starts_with("year").then(|| pair[0].parse().ok())?)
}

/// Returns every reason this document is not publishable. Empty means it is.
///
/// The publish pipeline treats a non-empty result as fatal and refuses to write
/// artifacts. Unit tests deliberately do not, so that work can continue against
/// a document with known gaps.
///
/// `as_of` is the reference date for derived values. It is a parameter rather
/// than a clock read so this stays pure and testable.
pub fn validate(resume: &Resume, as_of: ImpreciseDate) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if resume.schema_version != SCHEMA_VERSION {
        issues.push(ValidationIssue::new(
            "schemaVersion",
            format!(
                "document is version {}, this build understands {SCHEMA_VERSION}",
                resume.schema_version
            ),
        ));
    }

    let profile = &resume.profile;
    issues.extend(issue_for("profile.fullName".into(), &profile.full_name));
    issues.extend(issue_for("profile.headline".into(), &profile.headline));
    issues.extend(issue_for("profile.email".into(), &profile.email));
    issues.extend(issue_for("profile.summary".into(), &profile.summary));

    for (i, link) in profile.links.iter().enumerate() {
        issues.extend(issue_for(format!("profile.links[{i}].url"), &link.url));
    }

    for role in &resume.roles {
        let slug = &role.slug;
        issues.extend(issue_for(format!("roles.{slug}.employer"), &role.employer));
        issues.extend(issue_for(format!("roles.{slug}.title"), &role.title));

        if let Some(arrangement) = &role.arrangement {
            issues.extend(issue_for(format!("roles.{slug}.arrangement"), arrangement));
        }

        if role.highlights.is_empty() {
            issues.push(ValidationIssue::new(
                format!("roles.{slug}.highlights"),
                "no highlights",
            ));
        }

        for (i, highlight) in role.highlights.iter().enumerate() {
            issues.extend(issue_for(
                format!("roles.{slug}.highlights[{i}]"),
                highlight,
            ));
        }
    }

    for group in &resume.skills {
        for (i, item) in group.items.iter().enumerate() {
            issues.extend(issue_for(format!("skills.{}.items[{i}]", group.slug), item));
        }
    }

    for project in &resume.projects {
        issues.extend(issue_for(
            format!("projects.{}.summary", project.slug),
            &project.summary,
        ));

        if let Some(url) = &project.url {
            issues.extend(issue_for(format!("projects.{}.url", project.slug), url));
        }

        if let Some(repo) = &project.repo {
            issues.extend(issue_for(format!("projects.{}.repo", project.slug), repo));
        }
    }

    for cert in &resume.certifications {
        issues.extend(issue_for(
            format!("certifications.{}.issued", cert.slug),
            &cert.issued,
        ));
    }

    // The old site's summary claimed ten years against a 2006 start and went
    // stale unnoticed. If the prose states a number, the roles have to agree
    // with it, or the document does not publish.
    if let Some(stated) = stated_years(&profile.summary) {
        let derived = resume.experience_years(as_of);
        if stated != derived {
            issues.push(ValidationIssue::new(
                "profile.summary",
                format!("prose claims {stated} years; the roles compute to {derived}"),
            ));
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load;

    const AS_OF: ImpreciseDate = ImpreciseDate::YearMonth(2026, 10);

    #[test]
    fn detects_the_known_placeholders() {
        let resume = load().unwrap();
        let issues = validate(&resume, AS_OF);

        // The document is intentionally incomplete right now. This asserts the
        // detector works, and documents what still needs filling in.
        assert!(
            issues.iter().any(|i| i.path == "roles.nipr.title"),
            "expected the unresolved job title to be caught, got: {issues:#?}"
        );
        assert!(issues.iter().all(|i| i.detail != "is empty"));
    }

    #[test]
    fn recognizes_bracketed_text() {
        assert!(is_placeholder("[Current Title]"));
        assert!(is_placeholder("deployed across [number] venues"));
        assert!(!is_placeholder("Senior Software Engineer"));
    }

    #[test]
    fn reads_a_stated_year_count() {
        assert_eq!(
            stated_years("Frontend engineer with 15 years building"),
            Some(15)
        );
        assert_eq!(stated_years("Nearly two decades of experience"), None);
    }

    #[test]
    fn summary_agrees_with_the_derived_total() {
        let resume = load().unwrap();
        let drift: Vec<_> = validate(&resume, AS_OF)
            .into_iter()
            .filter(|issue| issue.path == "profile.summary")
            .collect();

        assert!(
            drift.is_empty(),
            "summary prose disagrees with the computed total: {drift:#?}"
        );
    }
}
