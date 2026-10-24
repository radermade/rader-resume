//! Values computed from the document, never stored in it.
//!
//! The old site hardcoded "ten years" against a 2006 start and went stale.
//! Anything that changes with the calendar is derived here.
//!
//! There is no clock in this crate. Callers pass `as_of`, which keeps the
//! functions pure, the tests deterministic, and the date dependency out of a
//! crate that is meant to stay thin.

use std::cmp::Reverse;

use crate::{CareerBreak, ImpreciseDate, Resume, Role, RoleEnd};

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

impl ImpreciseDate {
    /// Months since year zero. A bare year is treated as January, which
    /// slightly undercounts tenure rather than inventing precision.
    pub fn as_months(self) -> u32 {
        let (year, month) = match self {
            Self::Year(year) => (year, 1),
            Self::YearMonth(year, month) => (year, month),
        };
        u32::from(year) * 12 + u32::from(month) - 1
    }

    /// Human-facing label: `"2006"` or `"Oct 2023"`.
    pub fn label(self) -> String {
        match self {
            Self::Year(year) => year.to_string(),
            Self::YearMonth(year, month) => {
                let name = MONTHS[usize::from(month - 1)];
                format!("{name} {year}")
            }
        }
    }
}

impl RoleEnd {
    /// Resolves `Current` against a reference date.
    pub fn resolve(self, as_of: ImpreciseDate) -> ImpreciseDate {
        match self {
            Self::Current => as_of,
            Self::Ended(date) => date,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Current => "Present".to_owned(),
            Self::Ended(date) => date.label(),
        }
    }
}

/// Formats a span for display, e.g. `"Oct 2023 – Present"`.
pub fn format_range(start: ImpreciseDate, end: RoleEnd) -> String {
    format!("{} – {}", start.label(), end.label())
}

/// Whole months between two dates, saturating at zero so a malformed range
/// cannot underflow.
fn span_months(start: ImpreciseDate, end: ImpreciseDate) -> u32 {
    end.as_months().saturating_sub(start.as_months())
}

impl Role {
    pub fn span_months(&self, as_of: ImpreciseDate) -> u32 {
        span_months(self.start, self.end.resolve(as_of))
    }

    pub fn range_label(&self) -> String {
        format_range(self.start, self.end)
    }
}

impl CareerBreak {
    pub fn span_months(&self, as_of: ImpreciseDate) -> u32 {
        span_months(self.start, self.end.resolve(as_of))
    }

    pub fn range_label(&self) -> String {
        format_range(self.start, self.end)
    }
}

/// One entry on the career timeline.
///
/// A career break is not a job, so it is a distinct variant rather than a
/// `Role` with empty fields. Renderers emit different markup per variant.
#[derive(Debug, Clone, Copy)]
pub enum TimelineEntry<'a> {
    Role(&'a Role),
    Break(&'a CareerBreak),
}

impl TimelineEntry<'_> {
    pub fn start(&self) -> ImpreciseDate {
        match self {
            Self::Role(role) => role.start,
            Self::Break(gap) => gap.start,
        }
    }

    pub fn end(&self) -> RoleEnd {
        match self {
            Self::Role(role) => role.end,
            Self::Break(gap) => gap.end,
        }
    }

    pub fn range_label(&self) -> String {
        format_range(self.start(), self.end())
    }
}

impl Resume {
    /// Roles and career breaks merged into one sequence, newest first.
    pub fn timeline(&self) -> Vec<TimelineEntry<'_>> {
        let mut entries: Vec<TimelineEntry<'_>> = self
            .roles
            .iter()
            .map(TimelineEntry::Role)
            .chain(self.career_breaks.iter().map(TimelineEntry::Break))
            .collect();

        entries.sort_by_key(|entry| Reverse(entry.start()));
        entries
    }

    /// Total months of employment, excluding career breaks.
    ///
    /// Assumes roles do not overlap. If overlapping roles are ever added this
    /// will double count, and the fix is to merge intervals before summing.
    pub fn experience_months(&self, as_of: ImpreciseDate) -> u32 {
        self.roles.iter().map(|role| role.span_months(as_of)).sum()
    }

    /// Whole years of employment, rounded down.
    pub fn experience_years(&self, as_of: ImpreciseDate) -> u32 {
        self.experience_months(as_of) / 12
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load;

    const AS_OF: ImpreciseDate = ImpreciseDate::YearMonth(2026, 10);

    #[test]
    fn labels_respect_precision() {
        assert_eq!(ImpreciseDate::Year(2006).label(), "2006");
        assert_eq!(ImpreciseDate::YearMonth(2023, 10).label(), "Oct 2023");
        assert_eq!(RoleEnd::Current.label(), "Present");
    }

    #[test]
    fn formats_a_range() {
        assert_eq!(
            format_range(ImpreciseDate::YearMonth(2023, 10), RoleEnd::Current),
            "Oct 2023 – Present"
        );
        assert_eq!(
            format_range(
                ImpreciseDate::Year(2016),
                RoleEnd::Ended(ImpreciseDate::Year(2018))
            ),
            "2016 – 2018"
        );
    }

    #[test]
    fn counts_months_across_precisions() {
        let role_months = span_months(
            ImpreciseDate::YearMonth(2023, 10),
            ImpreciseDate::YearMonth(2026, 10),
        );
        assert_eq!(role_months, 36);

        assert_eq!(
            span_months(ImpreciseDate::Year(2016), ImpreciseDate::Year(2018)),
            24
        );
    }

    #[test]
    fn excludes_the_career_break_from_experience() {
        let resume = load().unwrap();
        let breaks: u32 = resume
            .career_breaks
            .iter()
            .map(|gap| gap.span_months(AS_OF))
            .sum();

        assert_eq!(breaks, 36, "the 2018-2021 break is three years");

        // Pinned deliberately. If this number moves, either the content
        // changed or the span arithmetic did, and both are worth noticing.
        //
        // Note this is lower than the 17 years the summary prose claims. Bare
        // years count from January, so roles recorded without months
        // undercount by up to eleven months each. Honest range is 15 to 17.
        assert_eq!(resume.experience_years(AS_OF), 15);
    }

    #[test]
    fn timeline_is_newest_first_and_includes_breaks() {
        let resume = load().unwrap();
        let timeline = resume.timeline();

        assert_eq!(
            timeline.len(),
            resume.roles.len() + resume.career_breaks.len()
        );

        let starts: Vec<_> = timeline.iter().map(TimelineEntry::start).collect();
        let mut sorted = starts.clone();
        sorted.sort_by_key(|&start| Reverse(start));
        assert_eq!(starts, sorted);

        assert!(
            timeline
                .iter()
                .any(|e| matches!(e, TimelineEntry::Break(_)))
        );
    }
}
