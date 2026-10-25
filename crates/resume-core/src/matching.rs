//! The four dimension matchers and the aggregation step.
//!
//! Each matcher deliberately applies a different rule. If they shared one, the
//! `Parallel` state that runs them would be decoration over a loop, which is
//! the thing this project keeps refusing to build.
//!
//! The distinctions are not arbitrary:
//!
//! - A technology listed in a skills section is weaker evidence than one used
//!   in a role, but it is still evidence.
//! - A practice is demonstrated, not listed. "Testing" in a skills list is
//!   weaker than "raised coverage on a payments flow".
//! - Experience cannot come from a skills list at all. Claiming a skill is not
//!   experience of a domain.
//! - Seniority needs language of ownership, not topic overlap. Mentioning
//!   architecture is not evidence of owning it.

use crate::fit::{
    Assessment, Citation, Dimension, FitReport, Provenance, Requirement, RequirementKind, Tier,
    Verdict, corpus, search,
};
use crate::{ImpreciseDate, Resume};

/// Verbs and phrases that indicate scope rather than participation.
const OWNERSHIP_SIGNALS: &[&str] = &[
    "led",
    "lead",
    "owned",
    "own",
    "mentor",
    "mentored",
    "mentoring",
    "established",
    "responsible",
    "drove",
    "architect",
    "designed",
    "coordinat",
    "in charge",
    "training",
];

/// How many pieces of evidence to cite per requirement.
const MAX_CITATIONS: usize = 3;

fn contains_ownership(text: &str) -> bool {
    let lower = text.to_lowercase();
    OWNERSHIP_SIGNALS
        .iter()
        .any(|signal| lower.contains(signal))
}

fn assess(
    requirement: &Requirement,
    verdict: Verdict,
    citations: Vec<Citation>,
    rationale: String,
) -> Assessment {
    Assessment {
        requirement_id: requirement.id.clone(),
        verdict,
        citations,
        rationale,
    }
}

fn gap(requirement: &Requirement, rationale: &str) -> Assessment {
    assess(requirement, Verdict::Gap, Vec::new(), rationale.to_owned())
}

/// Matches one requirement under the rule for its dimension.
pub fn match_requirement(
    requirement: &Requirement,
    resume: &Resume,
    as_of: ImpreciseDate,
) -> Assessment {
    let entries = corpus(resume);
    let hits = search(&entries, &requirement.text, MAX_CITATIONS);

    match requirement.dimension {
        Dimension::Tech => {
            let Some(best) = hits.first() else {
                return gap(requirement, "Not mentioned anywhere in the resume.");
            };

            let from_role = hits.iter().any(|hit| hit.citation.is_from_role());
            let citations = hits.iter().map(|hit| hit.citation.clone()).collect();

            if from_role {
                let where_used = hits
                    .iter()
                    .find(|hit| hit.citation.is_from_role())
                    .map(|hit| hit.citation.describe(resume))
                    .unwrap_or_default();
                assess(
                    requirement,
                    Verdict::Strong,
                    citations,
                    format!("Used in work at {where_used}."),
                )
            } else {
                // Listed but not evidenced in a role. Honest partial rather
                // than a strong claim the resume does not support.
                assess(
                    requirement,
                    Verdict::Partial,
                    citations,
                    format!(
                        "Listed under {}, not tied to a specific role.",
                        best.citation.describe(resume)
                    ),
                )
            }
        }

        Dimension::Skills => {
            let demonstrated: Vec<_> = hits
                .iter()
                .filter(|hit| hit.citation.is_from_role())
                .collect();

            if !demonstrated.is_empty() {
                let where_used = demonstrated[0].citation.describe(resume);
                return assess(
                    requirement,
                    Verdict::Strong,
                    demonstrated
                        .iter()
                        .map(|hit| hit.citation.clone())
                        .collect(),
                    format!("Demonstrated at {where_used}."),
                );
            }

            match hits.first() {
                Some(hit) => assess(
                    requirement,
                    Verdict::Partial,
                    vec![hit.citation.clone()],
                    "Claimed as a practice but not shown in a specific role.".to_owned(),
                ),
                None => gap(requirement, "No evidence of this practice."),
            }
        }

        Dimension::Experience => {
            // Tenure requirements are arithmetic, not keyword overlap.
            if let Some(required) = requirement.min_years {
                let held = resume.experience_years(as_of);
                return if u32::from(required) <= held {
                    assess(
                        requirement,
                        Verdict::Strong,
                        // The arithmetic sums every role, so the claim cites
                        // every role. Citing one highlight instead would be
                        // cheaper and would assert that a bullet proves a
                        // span of years, which it does not.
                        resume
                            .roles
                            .iter()
                            .map(|role| Citation::Employment {
                                role: role.slug.clone(),
                            })
                            .collect(),
                        format!("{held} years of employment on record."),
                    )
                } else {
                    gap(
                        requirement,
                        &format!("Asks for {required} years; {held} on record."),
                    )
                };
            }

            // A skills list is not evidence of experience. Claiming a skill
            // and having worked in a domain are different assertions.
            let from_roles: Vec<_> = hits
                .iter()
                .filter(|hit| hit.citation.is_from_role())
                .collect();

            match from_roles.first() {
                Some(first) => assess(
                    requirement,
                    Verdict::Strong,
                    from_roles.iter().map(|hit| hit.citation.clone()).collect(),
                    format!("Evidenced by work at {}.", first.citation.describe(resume)),
                ),
                None => gap(
                    requirement,
                    "Not evidenced by any role. A skills listing does not establish this.",
                ),
            }
        }

        Dimension::Seniority => {
            // Topic overlap is not enough. The evidence has to describe scope.
            let with_scope: Vec<_> = hits
                .iter()
                .filter(|hit| hit.citation.is_from_role() && contains_ownership(hit.text))
                .collect();

            if let Some(first) = with_scope.first() {
                return assess(
                    requirement,
                    Verdict::Strong,
                    with_scope.iter().map(|hit| hit.citation.clone()).collect(),
                    format!("Scope evidenced at {}.", first.citation.describe(resume)),
                );
            }

            // Mentioned, but described as participation rather than ownership.
            match hits.iter().find(|hit| hit.citation.is_from_role()) {
                Some(hit) => assess(
                    requirement,
                    Verdict::Partial,
                    vec![hit.citation.clone()],
                    "Related work is described, but not in terms of ownership.".to_owned(),
                ),
                None => gap(requirement, "No evidence of this level of scope."),
            }
        }
    }
}

/// Runs one `Parallel` branch: every requirement for a single dimension.
pub fn match_dimension(
    dimension: Dimension,
    requirements: &[Requirement],
    resume: &Resume,
    as_of: ImpreciseDate,
) -> Vec<Assessment> {
    requirements
        .iter()
        .filter(|requirement| requirement.dimension == dimension)
        .map(|requirement| match_requirement(requirement, resume, as_of))
        .collect()
}

/// Merges branch output into one report.
///
/// Branch results arrive in whatever order the `Parallel` state returns them,
/// so assessments are reordered to follow the requirement order. A report that
/// reshuffles on every run would be impossible to diff.
pub fn aggregate(
    requirements: Vec<Requirement>,
    branches: Vec<Vec<Assessment>>,
    provenance: Provenance,
    unmatched_terms: usize,
) -> FitReport {
    let flat: Vec<Assessment> = branches.into_iter().flatten().collect();

    let assessments: Vec<Assessment> = requirements
        .iter()
        .filter_map(|requirement| {
            flat.iter()
                .find(|assessment| assessment.requirement_id == requirement.id)
                .cloned()
        })
        .collect();

    // A branch that failed contributes no assessments, so a dimension with
    // requirements that came back short was not fully evaluated. Derived by
    // comparing what was asked against what came back, rather than trusting a
    // flag from the orchestration layer, so it is correct whether a branch
    // failed, timed out, or was never invoked.
    let incomplete_dimensions = Dimension::ALL
        .into_iter()
        .filter(|dimension| {
            let wanted = requirements
                .iter()
                .filter(|requirement| requirement.dimension == *dimension)
                .count();

            let assessed = assessments
                .iter()
                .filter(|assessment| {
                    requirements.iter().any(|requirement| {
                        requirement.id == assessment.requirement_id
                            && requirement.dimension == *dimension
                    })
                })
                .count();

            wanted > 0 && assessed < wanted
        })
        .collect();

    FitReport {
        requirements,
        assessments,
        provenance,
        incomplete_dimensions,
        unmatched_terms,
    }
}

impl FitReport {
    pub fn tier(&self) -> Tier {
        Tier::from_score(self.score())
    }

    pub fn matched(&self) -> impl Iterator<Item = &Assessment> {
        self.assessments
            .iter()
            .filter(|assessment| assessment.verdict != Verdict::Gap)
    }

    /// Three to five things to lead with.
    ///
    /// The role highlights that satisfy the most must-have requirements,
    /// deduplicated by role so the answer is not five bullets from one job.
    /// This is the output worth building the feature for: it answers "what do
    /// I say first", which no job board does.
    pub fn emphasis(&self, resume: &Resume) -> Vec<String> {
        let must_have: Vec<&str> = self
            .requirements
            .iter()
            .filter(|requirement| requirement.kind == RequirementKind::MustHave)
            .map(|requirement| requirement.id.as_str())
            .collect();

        let mut tally: Vec<(Citation, usize)> = Vec::new();

        for assessment in self.matched() {
            if !must_have.contains(&assessment.requirement_id.as_str()) {
                continue;
            }

            for citation in &assessment.citations {
                if !citation.is_from_role() {
                    continue;
                }
                match tally.iter_mut().find(|(seen, _)| seen == citation) {
                    Some((_, count)) => *count += 1,
                    None => tally.push((citation.clone(), 1)),
                }
            }
        }

        tally.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

        let mut used_roles: Vec<String> = Vec::new();
        let mut points = Vec::new();

        for (citation, _) in tally {
            let Citation::RoleHighlight { role, .. } = &citation else {
                continue;
            };
            if used_roles.contains(&role.to_string()) {
                continue;
            }
            if let Some(text) = citation.resolve(resume) {
                used_roles.push(role.to_string());
                points.push(text.to_owned());
            }
            if points.len() == 5 {
                break;
            }
        }

        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fit::{DETERMINISTIC_METHOD, JobDescription};
    use crate::load;
    use crate::taxonomy::{Taxonomy, extract};

    const AS_OF: ImpreciseDate = ImpreciseDate::YearMonth(2026, 10);

    fn requirement(id: &str, dimension: Dimension, kind: RequirementKind) -> Requirement {
        Requirement {
            id: id.to_owned(),
            text: id.replace('-', " "),
            kind,
            dimension,
            min_years: None,
        }
    }

    #[test]
    fn experience_rejects_skills_list_evidence() {
        let resume = load().unwrap();

        let mut req = requirement(
            "kubernetes",
            Dimension::Experience,
            RequirementKind::MustHave,
        );
        req.text = "Kubernetes".to_owned();

        let assessment = match_requirement(&req, &resume, AS_OF);
        assert_eq!(assessment.verdict, Verdict::Gap);
        assert!(
            assessment
                .rationale
                .contains("skills listing does not establish")
        );
    }

    #[test]
    fn a_satisfied_tenure_requirement_still_produces_a_valid_report() {
        // Regression. A real posting says "5+ years", the tenure branch
        // answered Strong, and `Assessment::validate` rejected it as an
        // uncited claim, which surfaced to a visitor as a 500 and
        // "could not produce a report". Every test that asserted the verdict
        // skipped validation, and every test that validated used a fixture
        // with no tenure requirement, so the pair was never exercised
        // together.
        let resume = load().unwrap();
        let taxonomy = Taxonomy::load().unwrap();

        let job = JobDescription::new(
            "Senior Engineer. Requirements: 5+ years of experience, TypeScript.",
        )
        .unwrap();

        let extraction = extract(&taxonomy, &job);
        let requirements = extraction.requirements.clone();
        assert!(
            requirements.iter().any(|r| r.min_years == Some(5)),
            "the fixture must contain a tenure requirement"
        );

        let branches = Dimension::ALL
            .iter()
            .map(|dimension| match_dimension(*dimension, &requirements, &resume, AS_OF))
            .collect();

        let report = aggregate(
            requirements,
            branches,
            Provenance {
                method: DETERMINISTIC_METHOD.to_owned(),
                revision: taxonomy.revision.clone(),
                generated_at: "2026-10-25T00:00:00Z".to_owned(),
            },
            extraction.unmatched_terms,
        );

        report
            .validate(&resume)
            .expect("a tenure claim must cite the employment it rests on");
    }

    #[test]
    fn tenure_is_arithmetic_not_keywords() {
        let resume = load().unwrap();
        let held = resume.experience_years(AS_OF);

        let mut reachable = requirement("years", Dimension::Experience, RequirementKind::MustHave);
        reachable.min_years = Some(u8::try_from(held).unwrap());
        assert_eq!(
            match_requirement(&reachable, &resume, AS_OF).verdict,
            Verdict::Strong
        );

        let mut beyond = reachable.clone();
        beyond.min_years = Some(u8::try_from(held).unwrap() + 5);
        let assessment = match_requirement(&beyond, &resume, AS_OF);
        assert_eq!(assessment.verdict, Verdict::Gap);
        assert!(assessment.rationale.contains("on record"));
    }

    #[test]
    fn seniority_needs_ownership_language() {
        assert!(contains_ownership("Led onboarding and code review"));
        assert!(contains_ownership("Responsible for analysis and design"));
        assert!(!contains_ownership("Used JavaScript and Python"));
    }

    #[test]
    fn branches_only_handle_their_own_dimension() {
        let resume = load().unwrap();
        let requirements = vec![
            requirement("typescript", Dimension::Tech, RequirementKind::MustHave),
            requirement(
                "accessibility",
                Dimension::Skills,
                RequirementKind::MustHave,
            ),
        ];

        let tech = match_dimension(Dimension::Tech, &requirements, &resume, AS_OF);
        assert_eq!(tech.len(), 1);
        assert_eq!(tech[0].requirement_id, "typescript");
    }

    #[test]
    fn aggregation_follows_requirement_order_regardless_of_branch_order() {
        let resume = load().unwrap();
        let requirements = vec![
            requirement("typescript", Dimension::Tech, RequirementKind::MustHave),
            requirement(
                "accessibility",
                Dimension::Skills,
                RequirementKind::MustHave,
            ),
            requirement(
                "mentoring",
                Dimension::Seniority,
                RequirementKind::NiceToHave,
            ),
        ];

        // Branches returned out of order, as a Parallel state may.
        let branches = vec![
            match_dimension(Dimension::Seniority, &requirements, &resume, AS_OF),
            match_dimension(Dimension::Tech, &requirements, &resume, AS_OF),
            match_dimension(Dimension::Skills, &requirements, &resume, AS_OF),
        ];

        let report = aggregate(
            requirements,
            branches,
            Provenance {
                method: DETERMINISTIC_METHOD.to_owned(),
                revision: "1".to_owned(),
                generated_at: "2026-10-24T00:00:00Z".to_owned(),
            },
            0,
        );

        let order: Vec<&str> = report
            .assessments
            .iter()
            .map(|a| a.requirement_id.as_str())
            .collect();
        assert_eq!(order, ["typescript", "accessibility", "mentoring"]);
    }

    #[test]
    fn a_real_description_produces_a_report_with_emphasis() {
        let resume = load().unwrap();
        let taxonomy = Taxonomy::load().unwrap();

        let job = JobDescription::new(
            "Senior Frontend Engineer. \
             Requirements: TypeScript, React, REST APIs, accessibility, code review, \
             frontend experience, mentoring. \
             Nice to have: AWS Lambda, Kubernetes.",
        )
        .unwrap();

        let extraction = extract(&taxonomy, &job);
        let requirements = extraction.requirements.clone();
        assert!(!requirements.is_empty());

        let branches = Dimension::ALL
            .iter()
            .map(|dimension| match_dimension(*dimension, &requirements, &resume, AS_OF))
            .collect();

        let report = aggregate(
            requirements,
            branches,
            Provenance {
                method: DETERMINISTIC_METHOD.to_owned(),
                revision: taxonomy.revision.clone(),
                generated_at: "2026-10-24T00:00:00Z".to_owned(),
            },
            extraction.unmatched_terms,
        );

        assert_eq!(report.assessments.len(), report.requirements.len());
        report
            .validate(&resume)
            .expect("every citation must resolve");

        let emphasis = report.emphasis(&resume);
        assert!(
            !emphasis.is_empty(),
            "should surface something to lead with"
        );
        assert!(emphasis.len() <= 5);

        // Deduplicated by role: no two emphasis points from the same job.
        let mut unique = emphasis.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), emphasis.len());

        // Kubernetes is not on the resume, so it had better be a gap.
        assert!(report.gaps().count() > 0);
    }
}
