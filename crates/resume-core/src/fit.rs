//! Job description fit analysis: the domain types and the matching logic.
//!
//! Deliberately free of any model call. Everything here is pure and testable,
//! which is what lets the Bedrock tasks be thin wrappers around logic that was
//! already proven without spending money or waiting on a network round trip.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Resume, Slug};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FitError {
    #[error("citation does not resolve against the document: {0:?}")]
    DanglingCitation(Citation),
    #[error("a non-gap verdict needs at least one citation")]
    UncitedClaim,
    #[error("job description is empty")]
    EmptyInput,
    #[error("job description is {0} bytes, over the {MAX_INPUT_BYTES} byte limit")]
    InputTooLarge(usize),
}

/// Hard cap on visitor-supplied text, enforced before any model call.
///
/// An unauthenticated endpoint that triggers a model call is a cost
/// amplification target, and token count scales with input length. Generous
/// for a job description, which is rarely over a few thousand bytes.
pub const MAX_INPUT_BYTES: usize = 32 * 1024;

/// A validated, normalized job description.
///
/// Constructing one is the only way to get text into the workflow, which makes
/// the size cap impossible to forget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobDescription {
    normalized: String,
    fingerprint: String,
}

impl JobDescription {
    pub fn new(raw: &str) -> Result<Self, FitError> {
        if raw.len() > MAX_INPUT_BYTES {
            return Err(FitError::InputTooLarge(raw.len()));
        }

        // Collapsing whitespace means two pastes of the same posting that
        // differ only in line wrapping produce the same fingerprint, and so
        // collide in the cache instead of each costing a model call.
        let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");

        if normalized.is_empty() {
            return Err(FitError::EmptyInput);
        }

        let fingerprint = crate::fingerprint(normalized.as_bytes());
        Ok(Self {
            normalized,
            fingerprint,
        })
    }

    pub fn text(&self) -> &str {
        &self.normalized
    }

    /// Cache key and Step Functions execution name.
    ///
    /// Using it as the execution name means duplicate submissions collide at
    /// the service level rather than needing application locking, the same
    /// trick the publish pipeline uses with commit SHAs. Hex, so it satisfies
    /// the execution name character rules.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

/// A versioned prompt, stored as content in git rather than as a string
/// literal buried in a handler.
#[derive(Debug, Clone, Copy)]
pub struct Prompt {
    pub version: &'static str,
    template: &'static str,
}

impl Prompt {
    /// Substitutes the job description into the single placeholder.
    ///
    /// The template places the data between explicit markers and states that
    /// the span is untrusted, which is the structural half of the injection
    /// defense. The other half is schema validation of whatever comes back.
    pub fn render(&self, job: &JobDescription) -> String {
        self.template.replace("{{job_description}}", job.text())
    }
}

pub const EXTRACT_REQUIREMENTS: Prompt = Prompt {
    version: "extract-requirements.v1",
    template: include_str!("../../../content/prompts/extract-requirements.v1.md"),
};

pub const MATCH_REQUIREMENT: Prompt = Prompt {
    version: "match-requirement.v1",
    template: include_str!("../../../content/prompts/match-requirement.v1.md"),
};

impl Prompt {
    /// Renders the matching prompt for one requirement against candidate
    /// evidence.
    ///
    /// Evidence is presented with opaque identifiers rather than slugs and
    /// indices, so the model picks from a closed list. Mapping identifiers back
    /// to citations happens here, not in the model, which removes a whole class
    /// of fabrication.
    pub fn render_match(&self, requirement: &Requirement, evidence: &[&CorpusEntry<'_>]) -> String {
        let listed = evidence
            .iter()
            .enumerate()
            .map(|(i, entry)| format!("e{i}: {}", entry.text))
            .collect::<Vec<_>>()
            .join("\n");

        self.template
            .replace("{{requirement}}", &requirement.text)
            .replace("{{evidence}}", &listed)
    }
}

/// Resolves the identifiers from a matching response back into citations.
///
/// Anything the model returns that is not in the evidence list it was given is
/// dropped rather than trusted. Combined with
/// [`Assessment::validate`], which rejects an uncited non-gap verdict, a model
/// that invents evidence produces a rejected report rather than a plausible
/// lie.
pub fn resolve_evidence_ids(ids: &[String], evidence: &[&CorpusEntry<'_>]) -> Vec<Citation> {
    ids.iter()
        .filter_map(|id| {
            let index: usize = id.strip_prefix('e')?.parse().ok()?;
            evidence.get(index).map(|entry| entry.citation.clone())
        })
        .collect()
}

/// What produced a result, recorded so an output can be explained months later.
///
/// Field names are method-agnostic on purpose. For the deterministic path
/// `method` is `"deterministic-taxonomy"` and `revision` is the taxonomy
/// revision; for a model path they would be the model ID and the prompt
/// version. The report should not have to change shape if the method does.
///
/// No clock in this crate, so `generated_at` is supplied by the caller.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    pub method: String,
    pub revision: String,
    pub generated_at: String,
}

/// The method identifier for the deterministic path.
pub const DETERMINISTIC_METHOD: &str = "deterministic-taxonomy";

/// Wrapper for extraction output. A top-level object rather than a bare array,
/// because that is what model structured-output APIs expect.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequirementSet {
    pub requirements: Vec<Requirement>,
}

/// JSON Schema constraining the extraction call, generated from the same type
/// the response deserializes into. Two copies of a schema diverge.
pub fn requirements_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(RequirementSet)).expect("schema is serializable")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RequirementKind {
    MustHave,
    NiceToHave,
}

/// Which matcher owns a requirement.
///
/// This is what makes the `Parallel` state meaningful. Without it, four
/// branches would run identical logic over identical input, and the
/// orchestration would be decoration. Each dimension matches against a
/// different slice of the resume under a different rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Dimension {
    /// Languages, frameworks, platforms, tools.
    Tech,
    /// Practices: testing, accessibility, review, CI.
    Skills,
    /// Domain, industry, scale, tenure. Must be evidenced by a role, not a
    /// skills list.
    Experience,
    /// Ownership, mentoring, architecture, impact.
    Seniority,
}

impl Dimension {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tech => "Technology",
            Self::Skills => "Practices",
            Self::Experience => "Experience",
            Self::Seniority => "Scope and seniority",
        }
    }

    pub const ALL: [Self; 4] = [Self::Tech, Self::Skills, Self::Experience, Self::Seniority];
}

/// A single requirement extracted from a job description.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    /// Stable within one analysis, so matches can reference requirements
    /// without repeating their text.
    pub id: String,
    pub text: String,
    pub kind: RequirementKind,
    pub dimension: Dimension,
    /// Set for tenure requirements such as "5+ years of experience".
    ///
    /// An explicit field rather than re-parsing `text` in the matcher, so the
    /// number is extracted once where the parsing is tested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_years: Option<u8>,
}

/// A band over the score.
///
/// A bare number invites false precision. "Solid" is an honest summary of 68
/// in a way that "68%" is not, given the inputs are keyword overlap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    Strong,
    Solid,
    Partial,
    Weak,
}

impl Tier {
    pub fn from_score(score: u8) -> Self {
        match score {
            80..=100 => Self::Strong,
            60..=79 => Self::Solid,
            35..=59 => Self::Partial,
            _ => Self::Weak,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Strong => "Strong match",
            Self::Solid => "Solid match",
            Self::Partial => "Partial match",
            Self::Weak => "Weak match",
        }
    }
}

/// A pointer into the resume document.
///
/// Citations are structural rather than quoted text. A model that invents an
/// accomplishment produces a citation that fails to resolve, so fabrication is
/// caught by validation instead of by a human noticing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Citation {
    RoleHighlight {
        #[schemars(with = "String")]
        role: Slug,
        index: usize,
    },
    /// A whole employment record, rather than a line within it.
    ///
    /// Tenure is arithmetic over role dates, so there is no bullet to quote.
    /// Without this shape, a satisfied "5+ years" requirement was a `Strong`
    /// verdict with no citations, `Assessment::validate` correctly rejected it
    /// as an uncited claim, and the visitor got a 500. The fix is to give the
    /// claim something real to point at rather than to exempt it from the
    /// rule, because the rule is the only thing standing between a report and
    /// a plausible invention.
    Employment {
        #[schemars(with = "String")]
        role: Slug,
    },
    SkillItem {
        #[schemars(with = "String")]
        group: Slug,
        index: usize,
    },
    ProjectHighlight {
        #[schemars(with = "String")]
        project: Slug,
        index: usize,
    },
}

impl Citation {
    /// Human description of where the evidence comes from, used in rationales.
    pub fn describe(&self, resume: &Resume) -> String {
        match self {
            Self::RoleHighlight { role, .. } | Self::Employment { role } => resume
                .roles
                .iter()
                .find(|candidate| &candidate.slug == role)
                .map_or_else(|| "a role".to_owned(), |found| found.employer.clone()),
            Self::SkillItem { group, .. } => resume
                .skills
                .iter()
                .find(|candidate| &candidate.slug == group)
                .map_or_else(|| "skills".to_owned(), |found| found.title.clone()),
            Self::ProjectHighlight { project, .. } => resume
                .projects
                .iter()
                .find(|candidate| &candidate.slug == project)
                .map_or_else(|| "a project".to_owned(), |found| found.name.clone()),
        }
    }

    /// Whether this evidence comes from employment rather than a claimed list.
    pub fn is_from_role(&self) -> bool {
        matches!(self, Self::RoleHighlight { .. } | Self::Employment { .. })
    }

    /// Returns the cited text, or `None` if the citation does not resolve.
    pub fn resolve<'a>(&self, resume: &'a Resume) -> Option<&'a str> {
        match self {
            Self::RoleHighlight { role, index } => resume
                .roles
                .iter()
                .find(|candidate| &candidate.slug == role)?
                .highlights
                .get(*index)
                .map(String::as_str),
            // Resolves to the employer, which is the thing being asserted to
            // exist. A date range would be more literal but it is computed
            // rather than stored, and a citation has to point at something
            // actually in the document to be worth checking.
            Self::Employment { role } => resume
                .roles
                .iter()
                .find(|candidate| &candidate.slug == role)
                .map(|found| found.employer.as_str()),
            Self::SkillItem { group, index } => resume
                .skills
                .iter()
                .find(|candidate| &candidate.slug == group)?
                .items
                .get(*index)
                .map(String::as_str),
            Self::ProjectHighlight { project, index } => resume
                .projects
                .iter()
                .find(|candidate| &candidate.slug == project)?
                .highlights
                .get(*index)
                .map(String::as_str),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    Strong,
    Partial,
    Gap,
}

impl Verdict {
    fn weight(self) -> f32 {
        match self {
            Self::Strong => 1.0,
            Self::Partial => 0.5,
            Self::Gap => 0.0,
        }
    }
}

/// One requirement, assessed.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Assessment {
    pub requirement_id: String,
    pub verdict: Verdict,
    pub citations: Vec<Citation>,
    pub rationale: String,
}

impl Assessment {
    /// Every claim of experience must point at something real.
    ///
    /// A `Gap` needs no citation, because there is nothing to cite. Anything
    /// else without a resolving citation is a hallucination and is rejected
    /// rather than displayed.
    pub fn validate(&self, resume: &Resume) -> Result<(), FitError> {
        for citation in &self.citations {
            if citation.resolve(resume).is_none() {
                return Err(FitError::DanglingCitation(citation.clone()));
            }
        }

        if self.verdict != Verdict::Gap && self.citations.is_empty() {
            return Err(FitError::UncitedClaim);
        }

        Ok(())
    }
}

/// A searchable view of the resume, with each entry carrying the citation that
/// points back at it.
#[derive(Debug, Clone)]
pub struct CorpusEntry<'a> {
    pub citation: Citation,
    pub text: &'a str,
}

/// Flattens the document into citable text.
///
/// Unfilled placeholders are excluded. Without this, a skills entry reading
/// `[Containers, Kubernetes, or other orchestration if applicable]` would
/// match a Kubernetes requirement and the report would claim a technology the
/// resume does not actually claim. Found by running a real job description
/// against the deployed workflow, which no unit test had covered.
pub fn corpus(resume: &Resume) -> Vec<CorpusEntry<'_>> {
    let roles = resume.roles.iter().flat_map(|role| {
        role.highlights
            .iter()
            .enumerate()
            .map(move |(index, text)| CorpusEntry {
                citation: Citation::RoleHighlight {
                    role: role.slug.clone(),
                    index,
                },
                text,
            })
    });

    let skills = resume.skills.iter().flat_map(|group| {
        group
            .items
            .iter()
            .enumerate()
            .map(move |(index, text)| CorpusEntry {
                citation: Citation::SkillItem {
                    group: group.slug.clone(),
                    index,
                },
                text,
            })
    });

    let projects = resume.projects.iter().flat_map(|project| {
        project
            .highlights
            .iter()
            .enumerate()
            .map(move |(index, text)| CorpusEntry {
                citation: Citation::ProjectHighlight {
                    project: project.slug.clone(),
                    index,
                },
                text,
            })
    });

    roles
        .chain(skills)
        .chain(projects)
        .filter(|entry| !crate::is_placeholder(entry.text))
        .collect()
}

/// Words too common to carry signal in a resume or job description.
const NOISE: &[&str] = &[
    "and",
    "the",
    "with",
    "for",
    "from",
    "that",
    "this",
    "into",
    "across",
    "using",
    "experience",
    "years",
    "work",
    "working",
    "team",
    "teams",
    "able",
    "strong",
    "ability",
];

pub(crate) fn terms(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '+' && c != '#')
        .filter(|word| word.len() > 2)
        .map(str::to_lowercase)
        .filter(|word| !NOISE.contains(&word.as_str()))
        .collect()
}

/// Ranks corpus entries by term overlap with the query.
///
/// Keyword matching rather than embeddings, deliberately: the corpus is one
/// resume, a few dozen entries. Embeddings would demonstrate more current
/// technique and buy nothing measurable at this size. Revisit if the corpus
/// grows or if recall proves poor against real job descriptions.
pub fn search<'a, 'c>(
    corpus: &'c [CorpusEntry<'a>],
    query: &str,
    limit: usize,
) -> Vec<&'c CorpusEntry<'a>> {
    let wanted = terms(query);
    if wanted.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(usize, &CorpusEntry<'a>)> = corpus
        .iter()
        .map(|entry| {
            let available = terms(entry.text);
            let hits = wanted
                .iter()
                .filter(|term| available.iter().any(|candidate| candidate == *term))
                .count();
            (hits, entry)
        })
        .filter(|(hits, _)| *hits > 0)
        .collect();

    scored.sort_by_key(|(hits, _)| std::cmp::Reverse(*hits));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, entry)| entry)
        .collect()
}

/// The complete analysis of one job description.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FitReport {
    pub requirements: Vec<Requirement>,
    pub assessments: Vec<Assessment>,
    pub provenance: Provenance,
    /// Dimensions whose requirements were not fully assessed, because the
    /// branch responsible for them failed.
    ///
    /// A single failed branch yields a partial report rather than no report,
    /// which is only defensible if the report says which part is missing. The
    /// score is computed over what was assessed, so without this a degraded
    /// run is indistinguishable from a clean one.
    #[serde(default)]
    pub incomplete_dimensions: Vec<Dimension>,
    /// Clauses in the description's required blocks that the taxonomy could
    /// not explain, carried from extraction so the report can state its own
    /// coverage.
    ///
    /// Defaulted rather than required, so a report serialized before this
    /// field existed still deserializes. It reads as "nothing missed", which
    /// is the wrong direction for an honesty signal, and is acceptable only
    /// because no such report is persisted anywhere.
    #[serde(default)]
    pub unmatched_terms: usize,
}

impl FitReport {
    /// Rejects the whole report if any assessment is unsupported. Partial
    /// credibility is not a thing: one fabricated citation discredits the rest.
    pub fn validate(&self, resume: &Resume) -> Result<(), FitError> {
        for assessment in &self.assessments {
            assessment.validate(resume)?;
        }
        Ok(())
    }

    pub fn gaps(&self) -> impl Iterator<Item = &Assessment> {
        self.assessments
            .iter()
            .filter(|assessment| assessment.verdict == Verdict::Gap)
    }

    /// A 0-100 fit score. Must-haves count double, because they do.
    pub fn score(&self) -> u8 {
        let kind_of = |id: &str| {
            self.requirements
                .iter()
                .find(|requirement| requirement.id == id)
                .map(|requirement| requirement.kind)
        };

        let (earned, possible) = self.assessments.iter().fold((0.0, 0.0), |(e, p), item| {
            let weight = match kind_of(&item.requirement_id) {
                Some(RequirementKind::MustHave) => 2.0,
                Some(RequirementKind::NiceToHave) => 1.0,
                None => 0.0,
            };
            (e + item.verdict.weight() * weight, p + weight)
        });

        if possible == 0.0 {
            return 0;
        }

        ((earned / possible) * 100.0).round() as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load;

    fn slug(value: &str) -> Slug {
        value.parse().unwrap()
    }

    #[test]
    fn corpus_covers_roles_skills_and_projects() {
        let resume = load().unwrap();
        let entries = corpus(&resume);

        assert!(entries.len() > 20);
        assert!(entries.iter().all(|entry| !entry.text.is_empty()));
        assert!(
            entries
                .iter()
                .any(|entry| matches!(entry.citation, Citation::ProjectHighlight { .. }))
        );
    }

    #[test]
    fn corpus_excludes_unfilled_placeholders() {
        let resume = load().unwrap();

        for entry in corpus(&resume) {
            assert!(
                !crate::is_placeholder(entry.text),
                "placeholder leaked into the corpus and would become a claim: {}",
                entry.text
            );
        }
    }

    #[test]
    fn a_placeholder_skill_cannot_be_matched() {
        let resume = load().unwrap();
        let entries = corpus(&resume);

        // The content currently carries "[Containers, Kubernetes, or other
        // orchestration if applicable]". Searching for Kubernetes must find
        // nothing, because the resume does not claim it.
        assert!(
            search(&entries, "Kubernetes", 5).is_empty(),
            "a bracketed placeholder must not satisfy a requirement"
        );
    }

    #[test]
    fn every_corpus_citation_resolves() {
        let resume = load().unwrap();

        for entry in corpus(&resume) {
            assert_eq!(
                entry.citation.resolve(&resume),
                Some(entry.text),
                "citation {:?} should resolve to its own text",
                entry.citation
            );
        }
    }

    #[test]
    fn search_finds_relevant_entries() {
        let resume = load().unwrap();
        let entries = corpus(&resume);

        let hits = search(&entries, "TypeScript React OpenAPI", 5);
        assert!(!hits.is_empty());
        assert!(
            hits.iter()
                .any(|hit| hit.text.contains("TypeScript") || hit.text.contains("OpenAPI"))
        );
    }

    #[test]
    fn search_ignores_noise_only_queries() {
        let resume = load().unwrap();
        let entries = corpus(&resume);

        assert!(search(&entries, "and the with for", 5).is_empty());
    }

    #[test]
    fn dangling_citations_are_rejected() {
        let resume = load().unwrap();

        let fabricated = Assessment {
            requirement_id: "r1".into(),
            verdict: Verdict::Strong,
            citations: vec![Citation::RoleHighlight {
                role: slug("nipr"),
                index: 999,
            }],
            rationale: "invented".into(),
        };

        assert!(matches!(
            fabricated.validate(&resume),
            Err(FitError::DanglingCitation(_))
        ));
    }

    #[test]
    fn claims_without_citations_are_rejected_unless_they_are_gaps() {
        let resume = load().unwrap();

        let uncited = Assessment {
            requirement_id: "r1".into(),
            verdict: Verdict::Partial,
            citations: vec![],
            rationale: "trust me".into(),
        };
        assert_eq!(uncited.validate(&resume), Err(FitError::UncitedClaim));

        let gap = Assessment {
            verdict: Verdict::Gap,
            ..uncited
        };
        assert_eq!(gap.validate(&resume), Ok(()));
    }

    #[test]
    fn normalization_makes_equivalent_pastes_collide() {
        let a = JobDescription::new("Senior   Engineer\n\nRust  and AWS").unwrap();
        let b = JobDescription::new("  Senior Engineer Rust and AWS  ").unwrap();

        assert_eq!(a.text(), b.text());
        assert_eq!(a.fingerprint(), b.fingerprint());

        let different = JobDescription::new("Senior Engineer Go and GCP").unwrap();
        assert_ne!(a.fingerprint(), different.fingerprint());
    }

    #[test]
    fn fingerprint_is_usable_as_an_execution_name() {
        let job = JobDescription::new("anything").unwrap();
        let name = job.fingerprint();

        assert!(!name.is_empty() && name.len() <= 80);
        assert!(name.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn input_limits_are_enforced_at_construction() {
        assert_eq!(JobDescription::new("   \n  "), Err(FitError::EmptyInput));

        let oversized = "x".repeat(MAX_INPUT_BYTES + 1);
        assert_eq!(
            JobDescription::new(&oversized),
            Err(FitError::InputTooLarge(MAX_INPUT_BYTES + 1))
        );
    }

    #[test]
    fn prompt_embeds_the_description_between_markers() {
        let job = JobDescription::new("Must know Rust").unwrap();
        let rendered = EXTRACT_REQUIREMENTS.render(&job);

        assert!(rendered.contains("---BEGIN JOB DESCRIPTION---"));
        assert!(rendered.contains("Must know Rust"));
        assert!(!rendered.contains("{{job_description}}"));
        assert_eq!(EXTRACT_REQUIREMENTS.version, "extract-requirements.v1");
    }

    #[test]
    fn extraction_schema_matches_the_deserialized_type() {
        let schema = requirements_schema();
        let properties = schema
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .expect("schema has properties");

        assert!(properties.contains_key("requirements"));
        assert_eq!(schema.get("additionalProperties"), Some(&false.into()));

        // The shape a producer is told to emit must be the shape we parse.
        let parsed: RequirementSet = serde_json::from_str(
            r#"{"requirements":[{"id":"rust","text":"Rust","kind":"mustHave","dimension":"tech"}]}"#,
        )
        .unwrap();
        assert_eq!(parsed.requirements.len(), 1);
        assert_eq!(parsed.requirements[0].dimension, Dimension::Tech);
    }

    #[test]
    fn match_prompt_lists_evidence_with_opaque_ids() {
        let resume = load().unwrap();
        let entries = corpus(&resume);
        let evidence = search(&entries, "TypeScript React", 3);

        let requirement = Requirement {
            id: "typescript".into(),
            text: "Strong TypeScript".into(),
            kind: RequirementKind::MustHave,
            dimension: Dimension::Tech,
            min_years: None,
        };

        let rendered = MATCH_REQUIREMENT.render_match(&requirement, &evidence);
        assert!(rendered.contains("Strong TypeScript"));
        assert!(rendered.contains("e0:"));
        assert!(!rendered.contains("{{requirement}}"));
        assert!(!rendered.contains("{{evidence}}"));
    }

    #[test]
    fn unknown_evidence_ids_are_dropped_not_trusted() {
        let resume = load().unwrap();
        let entries = corpus(&resume);
        let evidence = search(&entries, "TypeScript", 2);

        let ids = vec![
            "e0".to_owned(),
            "e99".to_owned(),
            "nonsense".to_owned(),
            "e1".to_owned(),
        ];
        let resolved = resolve_evidence_ids(&ids, &evidence);

        assert_eq!(resolved.len(), evidence.len().min(2));
        for citation in &resolved {
            assert!(citation.resolve(&resume).is_some());
        }
    }

    #[test]
    fn score_weights_must_haves_double() {
        let report = FitReport {
            unmatched_terms: 0,
            incomplete_dimensions: Vec::new(),
            provenance: Provenance {
                method: DETERMINISTIC_METHOD.into(),
                revision: "1".into(),
                generated_at: "2026-10-24T00:00:00Z".into(),
            },
            requirements: vec![
                Requirement {
                    id: "must".into(),
                    text: "Rust".into(),
                    kind: RequirementKind::MustHave,
                    dimension: Dimension::Tech,
                    min_years: None,
                },
                Requirement {
                    id: "nice".into(),
                    text: "Kubernetes".into(),
                    kind: RequirementKind::NiceToHave,
                    dimension: Dimension::Tech,
                    min_years: None,
                },
            ],
            assessments: vec![
                Assessment {
                    requirement_id: "must".into(),
                    verdict: Verdict::Strong,
                    citations: vec![],
                    rationale: String::new(),
                },
                Assessment {
                    requirement_id: "nice".into(),
                    verdict: Verdict::Gap,
                    citations: vec![],
                    rationale: String::new(),
                },
            ],
        };

        // 2 of 3 available weight.
        assert_eq!(report.score(), 67);
        assert_eq!(report.gaps().count(), 1);
    }
}
