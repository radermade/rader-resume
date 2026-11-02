//! Requirement extraction without a model.
//!
//! A taxonomy of canonical skills with aliases, plus section detection to tell
//! required from preferred. Deterministic, instant, free, and explainable.
//!
//! The tradeoff, stated plainly: anything outside the taxonomy is invisible,
//! and unusual phrasing will be missed. A model would generalize; this will
//! not. The mitigation is that the taxonomy is content in git, so a miss is a
//! one-line fix rather than a prompt tuning exercise, and `Extraction` reports
//! how many terms it could not place so the result is honest about coverage.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::fit::{Dimension, JobDescription, Requirement, RequirementKind};

/// The canonical taxonomy, embedded at compile time like the resume itself.
const SOURCE: &str = include_str!("../../../content/skills-taxonomy.json");

#[derive(Debug, Clone, Deserialize)]
pub struct Entry {
    pub id: String,
    pub label: String,
    pub dimension: Dimension,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Taxonomy {
    /// Bumped when entries change, recorded in report provenance so a result
    /// can be explained later.
    pub revision: String,
    pub entries: Vec<Entry>,
}

impl Taxonomy {
    pub fn load() -> Result<Self, serde_json::Error> {
        serde_json::from_str(SOURCE)
    }
}

/// What extraction produced, including what it could not.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Extraction {
    pub requirements: Vec<Requirement>,
    /// Clauses inside the description's required blocks that no taxonomy entry
    /// explained.
    ///
    /// Surfaced to the visitor rather than hidden, because a confident report
    /// over half the requirements is worse than an honest one. It is a count
    /// of clauses, not of real requirements: a clause may be prose rather than
    /// a requirement, so this is an upper bound on what was missed and should
    /// be worded that way wherever it is shown.
    pub unmatched_terms: usize,
}

/// Headings that mark a required block.
const REQUIRED_MARKERS: &[&str] = &[
    "requirements",
    "required",
    "must have",
    "must-have",
    "minimum qualifications",
    "basic qualifications",
    "what you need",
    "you have",
];

/// Headings that mark a preferred block.
const PREFERRED_MARKERS: &[&str] = &[
    "preferred",
    "nice to have",
    "nice-to-have",
    "bonus",
    "plus",
    "desired",
    "would be great",
];

/// Every occurrence of `needle` in `haystack` at term boundaries, as byte
/// spans, so "go" does not match "going" and "ts" does not match "its".
///
/// All occurrences rather than the first, because extraction wants the
/// earliest mention while coverage counting wants to know every clause a known
/// term appears in.
fn find_terms(haystack: &str, needle: &str) -> Vec<(usize, usize)> {
    if needle.is_empty() {
        return Vec::new();
    }

    let mut found = Vec::new();
    let mut from = 0;

    while let Some(offset) = haystack[from..].find(needle) {
        let start = from + offset;
        let end = start + needle.len();

        let before_ok = start == 0
            || !haystack[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '+' || c == '#');
        let after_ok = end >= haystack.len()
            || !haystack[end..]
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || c == '+' || c == '#');

        if before_ok && after_ok {
            found.push((start, end));
        }
        from = end;
    }

    found
}

/// Earliest boundary-respecting occurrence of `needle`.
fn find_term(haystack: &str, needle: &str) -> Option<usize> {
    find_terms(haystack, needle)
        .first()
        .map(|(start, _)| *start)
}

/// A required or preferred block: where it starts, where its heading ends, and
/// what it means.
///
/// The heading end matters for coverage counting. Without it the word
/// "requirements" is itself a clause that nothing in the taxonomy explains,
/// and every description would report one phantom unmatched requirement.
#[derive(Debug, Clone, Copy)]
struct Block {
    at: usize,
    heading_end: usize,
    kind: RequirementKind,
}

/// Byte offsets where a required or preferred block begins.
fn regions(lower: &str) -> Vec<Block> {
    let mut found: Vec<Block> = Vec::new();

    let mut push = |marker: &str, kind: RequirementKind| {
        if let Some(at) = find_term(lower, marker) {
            found.push(Block {
                at,
                heading_end: at + marker.len(),
                kind,
            });
        }
    };

    for marker in REQUIRED_MARKERS {
        push(marker, RequirementKind::MustHave);
    }
    for marker in PREFERRED_MARKERS {
        push(marker, RequirementKind::NiceToHave);
    }

    found.sort_by_key(|block| block.at);
    found
}

/// Which block a byte offset falls in.
///
/// Text before any marker is treated as preferred. Job descriptions open with
/// company boilerplate, and promoting that to a hard requirement would inflate
/// every score.
fn kind_at(regions: &[Block], offset: usize) -> RequirementKind {
    regions
        .iter()
        .rev()
        .find(|block| block.at <= offset)
        .map_or(RequirementKind::NiceToHave, |block| block.kind)
}

/// Finds a tenure requirement such as "5+ years" or "7 years of experience".
///
/// Returns the byte offset alongside the number so the required-versus-
/// preferred classification uses the same region logic as everything else.
fn find_min_years(lower: &str) -> Option<(usize, u8)> {
    let mut offset = 0;

    let tokens: Vec<(usize, &str)> = lower
        .split_whitespace()
        .map(|token| {
            let at = lower[offset..].find(token).map_or(offset, |i| offset + i);
            offset = at + token.len();
            (at, token)
        })
        .collect();

    tokens.windows(2).find_map(|pair| {
        if !pair[1].1.starts_with("year") {
            return None;
        }
        let number = pair[0].1.trim_end_matches('+').trim_end_matches('-');
        number.parse().ok().map(|years| (pair[0].0, years))
    })
}

/// Punctuation that still delimits a list item after whitespace collapsing.
///
/// Newlines and bullets are gone by the time text reaches here, because
/// `JobDescription` normalizes whitespace to make equivalent pastes collide.
/// What survives is the punctuation, which is enough: job descriptions
/// separate requirements with commas and periods far more often than with
/// layout alone.
const CLAUSE_BREAKS: &[char] = &[',', ';', '.', '|', '/', '(', ')', ':', '•'];

/// Splits the byte range `from..to` into clauses, as absolute byte spans.
fn clauses(lower: &str, from: usize, to: usize) -> Vec<(usize, usize)> {
    let span = &lower[from..to];
    let mut found = Vec::new();
    let mut start = 0;

    for (offset, character) in span.char_indices() {
        if CLAUSE_BREAKS.contains(&character) {
            if offset > start {
                found.push((from + start, from + offset));
            }
            start = offset + character.len_utf8();
        }
    }

    if start < span.len() {
        found.push((from + start, from + span.len()));
    }

    found
}

/// The spans covered by blocks the description marked as hard requirements,
/// each beginning after its heading.
fn required_spans(blocks: &[Block], len: usize) -> Vec<(usize, usize)> {
    blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.kind == RequirementKind::MustHave)
        .map(|(index, block)| {
            let end = blocks
                .get(index + 1)
                .map_or(len, |next| next.at)
                .max(block.heading_end);
            (block.heading_end, end)
        })
        .collect()
}

/// Counts clauses in the required blocks that no taxonomy entry explains.
///
/// This is the coverage number the report shows, and it only counts required
/// blocks on purpose. Scanning company boilerplate would report dozens of
/// unmatched "requirements" that were never requirements, and an honesty
/// signal that cries wolf gets ignored, which costs more than not having one.
///
/// A clause counts only if it carries a term with some signal, so "and" or a
/// stray "the" between two matched items is not an invisible requirement.
fn unmatched_clauses(lower: &str, blocks: &[Block], placed: &[(usize, usize)]) -> usize {
    required_spans(blocks, lower.len())
        .into_iter()
        .flat_map(|(from, to)| clauses(lower, from, to))
        .filter(|(from, to)| {
            let explained = placed.iter().any(|(at, _)| at >= from && at < to);
            !explained && !crate::fit::terms(&lower[*from..*to]).is_empty()
        })
        .count()
}

/// Extracts requirements by matching the taxonomy against the description.
///
/// Deterministic: the same text and taxonomy always produce the same
/// requirements in the same order, which is what makes the result cacheable
/// and the tests meaningful.
pub fn extract(taxonomy: &Taxonomy, job: &JobDescription) -> Extraction {
    let lower = job.text().to_lowercase();
    let blocks = regions(&lower);

    let mut requirements = Vec::new();

    // Every span a taxonomy term occupies, so coverage counting can tell a
    // clause it explained from one it did not.
    let mut placed: Vec<(usize, usize)> = Vec::new();

    for entry in &taxonomy.entries {
        // Earliest mention wins, and a term appearing in a required block
        // anywhere is a must-have even if it also appears in boilerplate.
        let mut best: Option<(usize, RequirementKind)> = None;

        for candidate in std::iter::once(&entry.label).chain(entry.aliases.iter()) {
            let occurrences = find_terms(&lower, &candidate.to_lowercase());

            // Every occurrence counts as explained, not only the one that
            // becomes the requirement. A term repeated under a later heading
            // would otherwise read as an unmatched requirement.
            placed.extend(occurrences.iter().copied());

            let Some((at, _)) = occurrences.first().copied() else {
                continue;
            };
            let kind = kind_at(&blocks, at);

            best = match best {
                Some((_, RequirementKind::MustHave)) => best,
                _ => Some((at, kind)),
            };
        }

        if let Some((at, kind)) = best {
            requirements.push((
                at,
                Requirement {
                    id: entry.id.clone(),
                    text: entry.label.clone(),
                    kind,
                    dimension: entry.dimension,
                    min_years: None,
                },
            ));
        }
    }

    // Tenure is not a taxonomy entry; it is a number in the prose.
    if let Some((at, years)) = find_min_years(&lower) {
        placed.push((at, at + 1));
        requirements.push((
            at,
            Requirement {
                id: "years-experience".to_owned(),
                text: format!("{years}+ years of experience"),
                kind: kind_at(&blocks, at),
                dimension: Dimension::Experience,
                min_years: Some(years),
            },
        ));
    }

    // Stable order: as they appear in the description.
    requirements.sort_by_key(|(at, _)| *at);

    Extraction {
        requirements: requirements.into_iter().map(|(_, req)| req).collect(),
        unmatched_terms: unmatched_clauses(&lower, &blocks, &placed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn taxonomy() -> Taxonomy {
        Taxonomy::load().expect("taxonomy must parse")
    }

    #[test]
    fn taxonomy_loads_with_unique_ids_and_all_dimensions() {
        let taxonomy = taxonomy();
        assert!(taxonomy.entries.len() > 30);

        let mut ids: Vec<&str> = taxonomy.entries.iter().map(|e| e.id.as_str()).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate taxonomy id");

        for dimension in [
            Dimension::Tech,
            Dimension::Skills,
            Dimension::Experience,
            Dimension::Seniority,
        ] {
            assert!(
                taxonomy.entries.iter().any(|e| e.dimension == dimension),
                "no entries for {dimension:?}"
            );
        }
    }

    #[test]
    fn term_matching_respects_boundaries() {
        assert!(find_term("we use go daily", "go").is_some());
        assert!(find_term("we are going", "go").is_none());
        assert!(find_term("its typescript", "ts").is_none());
        assert!(find_term("ts and js", "ts").is_some());
        assert!(find_term("c++ and rust", "c++").is_some());
    }

    #[test]
    fn required_and_preferred_sections_are_distinguished() {
        let job = JobDescription::new(
            "About us: we love React. \
             Requirements: TypeScript, REST APIs. \
             Nice to have: Kubernetes, Terraform.",
        )
        .unwrap();

        let extraction = extract(&taxonomy(), &job);
        let kind_of = |id: &str| {
            extraction
                .requirements
                .iter()
                .find(|r| r.id == id)
                .map(|r| r.kind)
        };

        assert_eq!(kind_of("typescript"), Some(RequirementKind::MustHave));
        assert_eq!(kind_of("rest"), Some(RequirementKind::MustHave));
        assert_eq!(kind_of("kubernetes"), Some(RequirementKind::NiceToHave));
        assert_eq!(kind_of("terraform"), Some(RequirementKind::NiceToHave));
    }

    #[test]
    fn boilerplate_before_any_marker_is_not_a_must_have() {
        let job = JobDescription::new("We are a React shop with great benefits.").unwrap();
        let extraction = extract(&taxonomy(), &job);

        assert_eq!(
            extraction
                .requirements
                .iter()
                .find(|r| r.id == "react")
                .map(|r| r.kind),
            Some(RequirementKind::NiceToHave),
            "company boilerplate must not inflate into a hard requirement"
        );
    }

    #[test]
    fn requirements_carry_dimensions_for_the_parallel_branches() {
        let job = JobDescription::new(
            "Requirements: TypeScript, accessibility, frontend experience, mentoring.",
        )
        .unwrap();

        let extraction = extract(&taxonomy(), &job);
        let dimension_of = |id: &str| {
            extraction
                .requirements
                .iter()
                .find(|r| r.id == id)
                .map(|r| r.dimension)
        };

        assert_eq!(dimension_of("typescript"), Some(Dimension::Tech));
        assert_eq!(dimension_of("accessibility"), Some(Dimension::Skills));
        assert_eq!(
            dimension_of("frontend-experience"),
            Some(Dimension::Experience)
        );
        assert_eq!(dimension_of("mentoring"), Some(Dimension::Seniority));
    }

    #[test]
    fn extraction_is_deterministic() {
        let job = JobDescription::new("Requirements: React, TypeScript, AWS Lambda.").unwrap();
        let taxonomy = taxonomy();

        let first = extract(&taxonomy, &job);
        let second = extract(&taxonomy, &job);

        let ids = |e: &Extraction| {
            e.requirements
                .iter()
                .map(|r| r.id.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&first), ids(&second));
    }

    #[test]
    fn tenure_requirements_are_parsed_as_numbers() {
        assert_eq!(find_min_years("5+ years of experience"), Some((0, 5)));
        assert_eq!(find_min_years("at least 7 years building"), Some((9, 7)));
        assert_eq!(find_min_years("many years of fun"), None);

        let job = JobDescription::new("Requirements: 8+ years of frontend experience.").unwrap();
        let extraction = extract(&taxonomy(), &job);

        let tenure = extraction
            .requirements
            .iter()
            .find(|r| r.id == "years-experience")
            .expect("tenure requirement");

        assert_eq!(tenure.min_years, Some(8));
        assert_eq!(tenure.kind, RequirementKind::MustHave);
        assert_eq!(tenure.dimension, Dimension::Experience);
    }

    #[test]
    fn nothing_matches_an_unrelated_description() {
        let job =
            JobDescription::new("Requirements: commercial driving licence, forklift.").unwrap();
        assert!(extract(&taxonomy(), &job).requirements.is_empty());
    }

    #[test]
    fn fully_covered_requirements_report_no_gap_in_coverage() {
        let job = JobDescription::new("Requirements: TypeScript, React, accessibility.").unwrap();

        let extraction = extract(&taxonomy(), &job);
        assert_eq!(extraction.requirements.len(), 3);
        assert_eq!(
            extraction.unmatched_terms, 0,
            "every clause was explained, so coverage must not claim a miss"
        );
    }

    #[test]
    fn requirements_outside_the_taxonomy_are_counted_not_hidden() {
        let job = JobDescription::new(
            "Requirements: TypeScript, commercial driving licence, \
             underwater welding certification.",
        )
        .unwrap();

        let extraction = extract(&taxonomy(), &job);

        assert_eq!(extraction.requirements.len(), 1);
        assert_eq!(
            extraction.unmatched_terms, 2,
            "a requirement the taxonomy cannot see must still be counted"
        );
    }

    #[test]
    fn the_section_heading_is_not_itself_an_unmatched_requirement() {
        let job = JobDescription::new("Requirements: TypeScript.").unwrap();
        assert_eq!(extract(&taxonomy(), &job).unmatched_terms, 0);
    }

    #[test]
    fn boilerplate_outside_required_blocks_is_not_counted() {
        // Marketing prose is not a requirement, and counting it would make the
        // coverage number meaningless by always being large.
        let job = JobDescription::new(
            "We are a fast growing company with excellent benefits, free lunches, \
             a dog friendly office and a ping pong table. \
             Requirements: TypeScript.",
        )
        .unwrap();

        assert_eq!(extract(&taxonomy(), &job).unmatched_terms, 0);
    }

    #[test]
    fn preferred_blocks_do_not_contribute_to_the_coverage_count() {
        let job = JobDescription::new(
            "Requirements: TypeScript. Nice to have: underwater welding certification.",
        )
        .unwrap();

        assert_eq!(
            extract(&taxonomy(), &job).unmatched_terms,
            0,
            "a missed nice-to-have does not undermine the report the way a \
             missed must-have does"
        );
    }

    #[test]
    fn coverage_counting_is_deterministic() {
        let job = JobDescription::new(
            "Requirements: TypeScript, quantum cryptography, React. \
             Nice to have: Kubernetes.",
        )
        .unwrap();
        let taxonomy = taxonomy();

        assert_eq!(
            extract(&taxonomy, &job).unmatched_terms,
            extract(&taxonomy, &job).unmatched_terms
        );
    }

    #[test]
    fn clause_splitting_covers_the_whole_span() {
        let text = "requirements: typescript, react (strong); rest apis.";
        let found = clauses(text, 0, text.len());

        assert!(found.iter().all(|(from, to)| from < to));
        assert!(
            found
                .iter()
                .any(|(from, to)| text[*from..*to].trim() == "typescript")
        );
        // Offsets are absolute and in range, which is what the coverage count
        // relies on when it slices back into the description.
        assert!(found.iter().all(|(_, to)| *to <= text.len()));
    }
}
