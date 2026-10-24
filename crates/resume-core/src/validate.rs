//! Content validation.
//!
//! Parsing proves the document matches the schema. Validation proves it is fit
//! to publish, which is a higher bar: a resume PDF reaching a recruiter with
//! `[Current Title]` in it is worse than any outage this project could have.

use crate::{Resume, SCHEMA_VERSION};

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

/// Returns every reason this document is not publishable. Empty means it is.
///
/// The publish pipeline treats a non-empty result as fatal and refuses to write
/// artifacts. Unit tests deliberately do not, so that work can continue against
/// a document with known gaps.
pub fn validate(resume: &Resume) -> Vec<ValidationIssue> {
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

    let mut check = |path: String, value: &str| {
        if value.trim().is_empty() {
            issues.push(ValidationIssue::new(path, "is empty"));
        } else if is_placeholder(value) {
            issues.push(ValidationIssue::new(path, "unresolved placeholder"));
        }
    };

    let profile = &resume.profile;
    check("profile.fullName".into(), &profile.full_name);
    check("profile.headline".into(), &profile.headline);
    check("profile.email".into(), &profile.email);
    check("profile.summary".into(), &profile.summary);
    for (i, link) in profile.links.iter().enumerate() {
        check(format!("profile.links[{i}].url"), &link.url);
    }

    for role in &resume.roles {
        let slug = &role.slug;
        check(format!("roles.{slug}.employer"), &role.employer);
        check(format!("roles.{slug}.title"), &role.title);

        if let Some(arrangement) = &role.arrangement {
            check(format!("roles.{slug}.arrangement"), arrangement);
        }

        if role.highlights.is_empty() {
            issues.push(ValidationIssue::new(
                format!("roles.{slug}.highlights"),
                "no highlights",
            ));
        }

        for (i, highlight) in role.highlights.iter().enumerate() {
            check(format!("roles.{slug}.highlights[{i}]"), highlight);
        }
    }

    for group in &resume.skills {
        for (i, item) in group.items.iter().enumerate() {
            check(format!("skills.{}.items[{i}]", group.slug), item);
        }
    }

    for project in &resume.projects {
        check(format!("projects.{}.summary", project.slug), &project.summary);
        if let Some(url) = &project.url {
            check(format!("projects.{}.url", project.slug), url);
        }
        if let Some(repo) = &project.repo {
            check(format!("projects.{}.repo", project.slug), repo);
        }
    }

    for cert in &resume.certifications {
        check(format!("certifications.{}.issued", cert.slug), &cert.issued);
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load;

    #[test]
    fn detects_the_known_placeholders() {
        let resume = load().unwrap();
        let issues = validate(&resume);

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
}
