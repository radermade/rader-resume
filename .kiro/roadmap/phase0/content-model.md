# Content Model v0 and Resume Revision

Source: `.kiro/old-resume/rader-resume/client/src/jobdata.json`, a local
snapshot of the three Firebase Realtime Database refs the old site read
(`/jobs`, `/info`, `/skills`). The Firebase project is deactivated, so the
snapshot is the extraction path. Nothing further is needed from it.

Output: `content/resume.json`, now the canonical document in git and embedded
into the binary at compile time by `resume-core`.

Assets worth salvaging from the old project: the portrait at
`client/src/images/me.png`, and the dark palette and Inconsolata type choice
recovered from `client/src/App.css`, both now recorded in
`.kiro/steering/design-standards.md`. Everything else in that repo stays where
it is.

## Normalization decisions

The old shape carried Realtime Database artifacts. RTDB has no array type, so
ordered lists were stored as single-element arrays wrapping maps keyed `"1"`
through `"n"`. Both `responsibilities` and `skill_list` had this shape.

- Keyed maps become ordered arrays. `responsibilities` is now `highlights:
  Vec<String>`; `skill_list` is now `items: Vec<String>`.
- `info` was an array of one. It is now a single `profile` object.
- Decorative angle brackets were baked into the data on every `job_name`,
  `skill_title`, and the `devspace` label. Stripped. That chrome belongs in the
  `maud` template, otherwise the same content can never render as a PDF without
  string surgery.
- Every role, skill group, and project now has a stable `slug` for routing and
  for keeping diffs clean when entries are reordered.
- Order comes from dates, not array position.
- `end_date: "current"` was a magic string. It is now a modelled variant.
- Dates are year-only for older roles and year-month where known. The parser
  accepts both precisions rather than forcing false precision onto 2006.
- Added `schemaVersion` per the Rust standards, and `projects` and
  `certifications` as first-class sections.

## Type sketch for `resume-core`

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Resume {
    pub schema_version: u32,
    pub profile: Profile,
    pub roles: Vec<Role>,
    pub career_breaks: Vec<CareerBreak>,
    pub skills: Vec<SkillGroup>,
    pub projects: Vec<Project>,
    pub certifications: Vec<Certification>,
    pub education: Vec<Education>,
}

/// Deliberately not a `Role`. A break has no employer and no title, so
/// modelling it as a job makes resume parsers and AI screeners read it as
/// three years of employment with no technical content. Renderers interleave
/// breaks into the timeline by date but emit different markup.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CareerBreak {
    pub slug: Slug,
    pub start: ImpreciseDate,
    pub end: RoleEnd,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Role {
    pub slug: Slug,
    pub employer: String,
    pub title: String,
    pub start: ImpreciseDate,
    pub end: RoleEnd,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrangement: Option<String>,
    pub highlights: Vec<String>,
}

/// Resume dates are honest about precision: 2006 is a year, not a month.
/// Wire form is a string, `"YYYY"` or `"YYYY-MM"`.
pub enum ImpreciseDate {
    Year(u16),
    YearMonth(u16, u8),
}

/// Wire form is `"current"` or an `ImpreciseDate` string.
pub enum RoleEnd {
    Current,
    Ended(ImpreciseDate),
}
```

Both of these carry hand-written `Serialize` and `Deserialize` impls delegating
to `Display` and `FromStr`, not derives. The wire form is a single string, which
no derive combination produces: an untagged enum over `Year(u16)` would expect a
JSON number, and the content file holds `"2021"`. Writing `FromStr` also puts
the parse errors somewhere testable, which matters because this is the one place
where a typo in authored content should fail validation loudly.

`deny_unknown_fields` applies here because this is authored content, where an
unknown key means a typo. Step Functions payload types do not get it.

Tenure sorting, total years of experience, and "current" formatting are derived
in code, never stored. Years of experience in particular was stale in the old
data (it claimed ten years against a 2006 start) and is a good first Phase 5
derived value.

## Revision approach

Target narrative: frontend developer who broadened into systems and DevOps,
returned to React with LLM integration and prompt engineering as the focus, and
is moving toward AWS solutions architecture.

Corrections applied from your notes:

- Disney is now framed as first exposure to React late in the tenure, with
  JavaScript, Python, and Lua as the primary work. Previously it read as a
  generic games role.
- Cinema Scene is now explicitly the primary React frontend role.
- Cerner is framed as the deliberate departure into systems and DevOps rather
  than more application development, which is what makes the arc legible.
- Cerner end date set to 2023-09 and the current role to 2023-10, closing the
  gap you described. Employer listed as "Cerner (Oracle Health)" since the
  rename happened mid-tenure. Title left exactly as the old data recorded it,
  "Interface Engineer II": employment verification compares that string, so a
  weaker-sounding accurate title beats an embellished one.
- The 2018 to 2021 break moved out of `roles` into `careerBreaks`. It is a real
  period on the timeline but it is not a job, and modelling it as one meant a
  parser would read it as three years of employment with no technical content.
- Current role leads with React plus LLM integration and prompt engineering
  practice.

Market-oriented edits:

- Headline and summary now state the arc in the first two lines, so a recruiter
  skimming does not have to infer it from job titles.
- Pre-2011 roles compressed to two or three lines. Recent roles carry the
  detail.
- The career break is two factual lines. The travel list and the yurt kept
  their charm but not their place; that personality belongs on the portfolio
  site, where it has room to land.
- Skills regrouped around the target role, led by AI and prompt engineering,
  with AWS and platform work ahead of tooling.
- Dropped as dated: CoffeeScript, ActionScript, Metaplace, Brackets, Fiddler,
  Heroku, and specific framework versions like "Python 3.7" and "Django".
  Omniture is now Adobe Analytics.
- This project is listed under `projects` as in-progress. It is the strongest
  available evidence for the architecture claim, which is exactly why it should
  ship.

## Placeholders to fill

Bracketed values in `resume.json` are things I will not invent.

- Current title and work arrangement. Employer is resolved: National Insurance
  Producer Registry (NIPR), the Kansas City not-for-profit handling insurance
  producer licensing data. Note the expansion is "Producer", not "Provider";
  a producer is an agent or broker in insurance terms.
- Whether the product area is LicenseHub, NIPR's producer licensing management
  platform, or something else. The highlight is bracketed with that guess.
- Contact email. Phone was deliberately dropped: the old `info` block put a
  real phone number in a client bundle served to the public. Steering forbids
  private contact details in `content/`. Use a form or an alias address.
- GitHub, LinkedIn, and portfolio URLs.
- Cerner: the actual deployment, environment, and monitoring stack. This is the
  weakest section, because the original bullets described process rather than
  technology, and this role is load-bearing for the systems engineering half of
  the story.
- Current role: one quantified outcome from the AI work. Adoption, delivery
  speed, defect rate, or cost all work. A prompt engineering claim without a
  number reads as enthusiasm.
- Cinema Scene venue count, Disney user scale, team sizes.
- Containers, Kubernetes, and SQL are bracketed because you have not mentioned
  them. Confirm or cut; do not list them hopefully.
- The AI section needs specifics. "Prompt engineering" alone is vaguer than
  current job descriptions: name the models and platforms you work against, and
  add retrieval-augmented generation, embeddings, evals, agentic workflows, or
  guardrails only where the work is real. Bedrock is absent and sits exactly at
  your AI-plus-AWS intersection.

Confirmed and added: Jest, Playwright, REST, web accessibility and WCAG, Agile
and Scrum, plus a tooling group restoring Git and GitHub, which fell out when
the skill groups were reorganized.

TypeScript resolved and added, along with Next.js and OpenAPI-driven type
generation. This materially changes the read of the frontend history. The
TypeScript thread runs from 2016 at Cinema Scene through current work at NIPR,
which spans the career break and makes the frontend foundation look continuous
rather than interrupted. Schema-first type generation is also the strongest
architecture-adjacent signal in the document: it is the same instinct as the
`serde` contract types in this project, which makes the Rust work read as an
extension of existing practice instead of a hobby detour.
- FAA Part 107 issue year.
- Education section is empty pending your input on whether to include it.

## Screening considerations

Applicant tracking systems are structured databases: they parse the resume into
fields, then recruiters search and rank. Missing terms mean you do not surface
in a search rather than that you were rejected. Above that layer, many larger
employers now run an AI screening agent that reads semantically and hands the
recruiter a ranked shortlist with a written rationale, which means narrative
framing helps and keyword stuffing is actively counterproductive.

Consequences for this document:

- Title fields carry disproportionate weight in recruiter search. They are the
  highest-value blanks in `resume.json`.
- Semantic matching rewards the arc stated plainly in `profile.summary`, which
  is why the summary leads with the trajectory rather than a list of adjectives.
- Keep the stated years of experience. It will not help with automated parsing,
  but it saves a human recruiter from doing arithmetic across a career break.
  Deliberate decision, not an oversight.
- There is no abstract keyword optimum. Ranking happens against a specific job
  description, which is the real argument for generating per-role variants from
  this document.
- No hidden text, no white-on-white keyword blocks. A screener that emits a
  rationale will describe exactly what it found.

Rendering matters as much as wording, and the PDF is the artifact that gets
submitted. Single column, real embedded text rather than images, standard
section headings, and nothing load-bearing inside a table or graphic.

## Recommendations

- An AWS Certified Solutions Architect Associate would do more for this pivot
  than any wording change on the page. The resume currently claims the
  direction and shows one in-progress project as support.
- The AWS skills group reflects this project's surface area. Keep it honest: if
  asked about Step Functions in an interview, the answer should be this
  codebase, not a tutorial.
- Consider a short "Selected Work" treatment on the site that lets the Cerner
  federal healthcare work carry weight without the detail a public page cannot
  include.
