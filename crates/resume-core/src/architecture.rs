//! The stack page's data: what this project runs on, and what it is built
//! from.
//!
//! Disclosure, not verification. Nothing here makes a network call, so there
//! is no outcome to be unreachable and nothing to orchestrate. That is the
//! whole reason this is not a provenance check: the provenance workflow exists
//! to verify claims against live sources, and this page only has to be true at
//! build time.
//!
//! Both halves are embedded with `include_str!`, which means the page shows
//! the manifests and diagram from the commit that produced the artifact. Same
//! git-as-truth property the footer SHA has, and it keeps the page a
//! pre-renderable static artifact rather than something that reads a file
//! system at request time.

use serde::{Deserialize, Serialize};

/// Schema version of the authored architecture document.
pub const SCHEMA_VERSION: u32 = 1;

/// One manifest, shown verbatim.
///
/// Verbatim is the point. A summarized dependency list is a claim; the actual
/// file is evidence, and the rationale comments in `Cargo.toml` say more about
/// how decisions get made here than a version table would.
#[derive(Debug, Clone, Copy)]
pub struct Manifest {
    pub name: &'static str,
    /// Why a reader should care, so the page is not just a wall of TOML.
    pub note: &'static str,
    pub body: &'static str,
    /// Hint for the `<code>` block's language label.
    pub language: &'static str,
}

/// The manifests worth publishing.
///
/// `infra/cdk.json` is deliberately absent. Its context carries a hosted zone
/// ID, which is a live AWS resource identifier rather than a secret but has no
/// upside in public, and a hand-maintained redaction list would fail open the
/// next time a context key is added. The architecture diagram covers what a
/// reader actually wanted from that file.
pub const MANIFESTS: &[Manifest] = &[
    Manifest {
        name: "Cargo.toml",
        note: "Workspace dependencies, the lint that forbids unsafe, and the release profile. \
               The comments are load-bearing: several of these pins exist because of a specific \
               failure.",
        body: include_str!("../../../Cargo.toml"),
        language: "toml",
    },
    Manifest {
        name: "infra/package.json",
        note: "The only Node dependency tree in the project, and it exists solely to run CDK. \
               Nothing in it reaches the browser.",
        body: include_str!("../../../infra/package.json"),
        language: "json",
    },
];

/// One component in the architecture diagram.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub label: String,
    /// One line on what it does and, where there was a choice, why it won.
    pub role: String,
    /// CloudFormation type, where this node is a resource in the site stack.
    ///
    /// `None` marks a component outside the stack, such as the visitor's
    /// browser or the certificate that has to live in another region. The CI
    /// check that compares this document against the synthesized template
    /// skips those, because there is nothing to compare them to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>,
}

impl Node {
    /// Whether this component lives outside the site stack.
    pub fn is_external(&self) -> bool {
        self.resource_type.is_none()
    }
}

/// A band in the diagram. Flow runs from the first tier to the last.
///
/// Tiers rather than explicit edges, because routing arrows in CSS means
/// hand-positioned coordinates, which is exactly what the old site did with
/// its hexagons and exactly what this is meant to avoid. Ordered bands convey
/// the same dataflow and lay out with grid.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tier {
    pub id: String,
    pub label: String,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Architecture {
    pub schema_version: u32,
    /// The one-paragraph version, for a reader who will not study a diagram.
    pub summary: String,
    pub tiers: Vec<Tier>,
}

const SOURCE: &str = include_str!("../../../content/architecture.json");

#[derive(Debug, thiserror::Error)]
pub enum ArchitectureError {
    #[error("architecture document is malformed: {0}")]
    Malformed(#[from] serde_json::Error),
    #[error("architecture document is schema version {found}, expected {SCHEMA_VERSION}")]
    WrongVersion { found: u32 },
}

/// Parses the embedded architecture document.
///
/// `deny_unknown_fields` applies, unlike the state machine payloads: this is
/// authored content, where an unknown field means a typo rather than an
/// orchestration layer adding something.
pub fn load() -> Result<Architecture, ArchitectureError> {
    let parsed: Architecture = serde_json::from_str(SOURCE)?;

    if parsed.schema_version != SCHEMA_VERSION {
        return Err(ArchitectureError::WrongVersion {
            found: parsed.schema_version,
        });
    }

    Ok(parsed)
}

impl Architecture {
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.tiers.iter().flat_map(|tier| tier.nodes.iter())
    }

    /// Distinct CloudFormation types the diagram claims, sorted.
    ///
    /// This is what the CI check compares against the synthesized template. It
    /// can only catch the diagram claiming something the stack does not run;
    /// it cannot prove the diagram is complete, which the page says plainly
    /// rather than leaving a reader to assume otherwise.
    pub fn claimed_resource_types(&self) -> Vec<&str> {
        let mut types: Vec<&str> = self
            .nodes()
            .filter_map(|node| node.resource_type.as_deref())
            .collect();
        types.sort_unstable();
        types.dedup();
        types
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_document_parses_at_the_expected_version() {
        let architecture = load().expect("architecture document must parse");

        assert_eq!(architecture.schema_version, SCHEMA_VERSION);
        assert!(!architecture.tiers.is_empty());
        assert!(!architecture.summary.is_empty());
    }

    #[test]
    fn node_ids_are_unique_across_tiers() {
        let architecture = load().unwrap();

        let mut ids: Vec<&str> = architecture.nodes().map(|node| node.id.as_str()).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), total, "duplicate node id");
    }

    #[test]
    fn every_node_explains_itself() {
        for node in load().unwrap().nodes() {
            assert!(!node.label.is_empty(), "{} has no label", node.id);
            assert!(
                !node.role.is_empty(),
                "{} has no role, so it is decoration rather than information",
                node.id
            );
            assert!(
                !crate::is_placeholder(&node.role),
                "{} still carries an authoring placeholder",
                node.id
            );
        }
    }

    #[test]
    fn claimed_types_look_like_cloudformation_types() {
        let architecture = load().unwrap();
        let claimed = architecture.claimed_resource_types();

        assert!(!claimed.is_empty());
        for kind in claimed {
            assert!(
                kind.starts_with("AWS::") && kind.matches("::").count() == 2,
                "{kind} is not a CloudFormation type, so the CI check could never match it"
            );
        }
    }

    #[test]
    fn external_components_are_the_ones_outside_the_stack() {
        let architecture = load().unwrap();

        // The browser and the us-east-1 certificate are not resources in this
        // stack and must not be asserted against its template.
        assert!(
            architecture
                .nodes()
                .any(|node| node.is_external() && node.id == "browser")
        );
        assert!(
            architecture
                .nodes()
                .filter(|node| !node.is_external())
                .count()
                > 5
        );
    }

    #[test]
    fn unknown_fields_are_a_typo_not_an_extension() {
        let result: Result<Architecture, _> =
            serde_json::from_str(r#"{"schemaVersion":1,"summary":"x","tiers":[],"tyers":"typo"}"#);
        assert!(
            result.is_err(),
            "authored content is strict; an unknown field is a mistake"
        );
    }

    #[test]
    fn manifests_are_embedded_and_non_empty() {
        assert!(!MANIFESTS.is_empty());

        for manifest in MANIFESTS {
            assert!(!manifest.body.is_empty(), "{} is empty", manifest.name);
            assert!(!manifest.note.is_empty(), "{} has no note", manifest.name);
        }

        // The workspace manifest is the interesting one, so prove it is really
        // the file and not a stub.
        let cargo = MANIFESTS
            .iter()
            .find(|manifest| manifest.name == "Cargo.toml")
            .expect("Cargo.toml is published");
        assert!(cargo.body.contains("[workspace]"));
        assert!(cargo.body.contains("unsafe_code"));
    }

    #[test]
    fn no_manifest_carries_an_account_identifier() {
        // cdk.json is excluded for exactly this reason. If a manifest ever
        // starts carrying a hosted zone id or an account number, this fails
        // before it reaches a public page.
        for manifest in MANIFESTS {
            assert!(
                !manifest.body.contains("hostedZoneId"),
                "{} carries a hosted zone id",
                manifest.name
            );

            for line in manifest.body.lines() {
                let digits: String = line.chars().filter(char::is_ascii_digit).collect();
                assert!(
                    digits.len() < 12,
                    "{} has a line with {} digits, which could be an account id: {line}",
                    manifest.name,
                    digits.len()
                );
            }
        }
    }
}
