//! Shared data model and logic for the living resume.
//!
//! This crate deliberately has no `aws-sdk-*` or `lambda_*` dependency. It is
//! the one place where resume content, validation, and rendering live, which
//! keeps all of it unit-testable without AWS.

mod dates;
mod validate;

pub use dates::{DateParseError, ImpreciseDate, RoleEnd};
pub use validate::{ValidationIssue, validate};

use serde::{Deserialize, Serialize};

/// Schema version of the authored content document.
pub const SCHEMA_VERSION: u32 = 1;

/// The canonical content document, embedded at compile time.
///
/// Git is the source of truth, and the binary is built from a commit, so
/// embedding ties the served content to the SHA that produced it with no
/// runtime fetch and no cold-start I/O.
pub const CONTENT: &str = include_str!("../../../content/resume.json");

/// Parses the embedded content document.
pub fn load() -> Result<Resume, serde_json::Error> {
    serde_json::from_str(CONTENT)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Resume {
    pub schema_version: u32,
    pub profile: Profile,
    pub roles: Vec<Role>,
    #[serde(default)]
    pub career_breaks: Vec<CareerBreak>,
    pub skills: Vec<SkillGroup>,
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub certifications: Vec<Certification>,
    #[serde(default)]
    pub education: Vec<Education>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub full_name: String,
    pub preferred_name: String,
    pub headline: String,
    pub location: String,
    pub email: String,
    pub links: Vec<Link>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Link {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Role {
    pub slug: String,
    pub employer: String,
    pub title: String,
    pub start: ImpreciseDate,
    pub end: RoleEnd,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrangement: Option<String>,
    pub highlights: Vec<String>,
}

/// Deliberately not a [`Role`]. A break has no employer and no title, so
/// modelling it as a job makes resume parsers read it as employment with no
/// technical content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CareerBreak {
    pub slug: String,
    pub start: ImpreciseDate,
    pub end: RoleEnd,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillGroup {
    pub slug: String,
    pub title: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Project {
    pub slug: String,
    pub name: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    pub summary: String,
    #[serde(default)]
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Certification {
    pub slug: String,
    pub name: String,
    pub issued: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Education {
    pub slug: String,
    pub institution: String,
    pub credential: String,
    pub completed: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_content_deserializes() {
        let resume = load().expect("content/resume.json must match the schema");
        assert_eq!(resume.schema_version, SCHEMA_VERSION);
        assert!(!resume.roles.is_empty());
    }

    #[test]
    fn roles_are_reverse_chronological() {
        let resume = load().unwrap();
        let starts: Vec<_> = resume.roles.iter().map(|role| role.start).collect();
        let mut sorted = starts.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(starts, sorted, "roles should be newest first");
    }

    #[test]
    fn slugs_are_unique() {
        let resume = load().unwrap();
        let mut slugs: Vec<_> = resume.roles.iter().map(|role| role.slug.as_str()).collect();
        let count = slugs.len();
        slugs.sort_unstable();
        slugs.dedup();
        assert_eq!(slugs.len(), count, "duplicate role slug");
    }
}
