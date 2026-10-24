//! Shared data model and logic for the living resume.
//!
//! This crate deliberately has no `aws-sdk-*` or `lambda_*` dependency. It is
//! the one place where resume content, validation, and rendering live, which
//! keeps all of it unit-testable without AWS.

mod dates;
mod derived;
pub mod render;
mod slug;
mod validate;

pub use dates::{DateParseError, ImpreciseDate, RoleEnd};
pub use derived::{TimelineEntry, format_range};
pub use render::{PageContext, page, robots_txt, sitemap_xml};
pub use slug::{Slug, SlugError};
pub use validate::{ValidationIssue, validate};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Schema version of the authored content document.
pub const SCHEMA_VERSION: u32 = 1;

/// JSON Schema for the content document, generated from the types themselves.
///
/// Derived rather than hand-written because Phase 6 needs schema-constrained
/// model output from these same definitions, and two copies of a schema
/// diverge.
///
/// Returns a `serde_json::Value` rather than a `schemars` type so the public
/// API does not move when schemars does.
///
/// `ImpreciseDate`, `RoleEnd`, and `Slug` appear as plain strings, which is
/// what they are on the wire. Their parse rules are not expressed as patterns
/// yet; add them if an external consumer needs them.
pub fn content_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(Resume)).expect("schema is serializable")
}

/// A stable fingerprint of the content, for cache validators.
///
/// FNV-1a: deterministic across builds and platforms, which a `DefaultHasher`
/// is not guaranteed to be. Not a security primitive and not used as one; it
/// only has to change when the content changes.
pub fn fingerprint(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Link {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Role {
    #[schemars(with = "String")]
    pub slug: Slug,
    pub employer: String,
    pub title: String,
    #[schemars(with = "String")]
    pub start: ImpreciseDate,
    #[schemars(with = "String")]
    pub end: RoleEnd,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrangement: Option<String>,
    pub highlights: Vec<String>,
}

/// Deliberately not a [`Role`]. A break has no employer and no title, so
/// modelling it as a job makes resume parsers read it as employment with no
/// technical content.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CareerBreak {
    #[schemars(with = "String")]
    pub slug: Slug,
    #[schemars(with = "String")]
    pub start: ImpreciseDate,
    #[schemars(with = "String")]
    pub end: RoleEnd,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillGroup {
    #[schemars(with = "String")]
    pub slug: Slug,
    pub title: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Project {
    #[schemars(with = "String")]
    pub slug: Slug,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Certification {
    #[schemars(with = "String")]
    pub slug: Slug,
    pub name: String,
    pub issued: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Education {
    #[schemars(with = "String")]
    pub slug: Slug,
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
        sorted.sort_by_key(|&start| std::cmp::Reverse(start));
        assert_eq!(starts, sorted, "roles should be newest first");
    }

    fn assert_unique(label: &str, mut slugs: Vec<&str>) {
        let count = slugs.len();
        slugs.sort_unstable();
        slugs.dedup();
        assert_eq!(slugs.len(), count, "duplicate slug in {label}");
    }

    #[test]
    fn slugs_are_unique_within_each_collection() {
        let resume = load().unwrap();

        assert_unique(
            "roles",
            resume.roles.iter().map(|it| it.slug.as_str()).collect(),
        );
        assert_unique(
            "careerBreaks",
            resume
                .career_breaks
                .iter()
                .map(|it| it.slug.as_str())
                .collect(),
        );
        assert_unique(
            "skills",
            resume.skills.iter().map(|it| it.slug.as_str()).collect(),
        );
        assert_unique(
            "projects",
            resume.projects.iter().map(|it| it.slug.as_str()).collect(),
        );
        assert_unique(
            "certifications",
            resume
                .certifications
                .iter()
                .map(|it| it.slug.as_str())
                .collect(),
        );
    }

    #[test]
    fn fingerprint_is_stable_and_content_sensitive() {
        assert_eq!(
            fingerprint(CONTENT.as_bytes()),
            fingerprint(CONTENT.as_bytes())
        );
        assert_ne!(fingerprint(b"a"), fingerprint(b"b"));
    }

    #[test]
    fn schema_describes_the_document() {
        let schema = content_schema();
        let properties = schema
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .expect("schema has properties");

        for field in [
            "schemaVersion",
            "profile",
            "roles",
            "careerBreaks",
            "skills",
        ] {
            assert!(properties.contains_key(field), "schema is missing {field}");
        }

        // Authored content is strict, and the schema should say so.
        assert_eq!(schema.get("additionalProperties"), Some(&false.into()));
    }
}
