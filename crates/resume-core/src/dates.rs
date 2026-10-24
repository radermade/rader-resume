//! Resume dates, honest about precision.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DateParseError {
    #[error("expected `YYYY` or `YYYY-MM`, got `{0}`")]
    Shape(String),
    #[error("month must be 1-12, got {0}")]
    Month(u8),
}

/// A resume date. 2006 is a year, not a month, and pretending otherwise
/// invents precision the source never had.
///
/// Wire form is a string, `"YYYY"` or `"YYYY-MM"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpreciseDate {
    Year(u16),
    YearMonth(u16, u8),
}

impl ImpreciseDate {
    /// Month is validated to 1-12, so a bare year sorting as month 0 cannot
    /// collide with a real `YearMonth`. That keeps `Ord` consistent with `Eq`.
    fn sort_key(self) -> (u16, u8) {
        match self {
            Self::Year(year) => (year, 0),
            Self::YearMonth(year, month) => (year, month),
        }
    }
}

impl Ord for ImpreciseDate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl PartialOrd for ImpreciseDate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl FromStr for ImpreciseDate {
    type Err = DateParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let shape = || DateParseError::Shape(s.to_owned());

        match s.split_once('-') {
            None => Ok(Self::Year(s.parse().map_err(|_| shape())?)),
            Some((year, month)) => {
                let year = year.parse().map_err(|_| shape())?;
                let month: u8 = month.parse().map_err(|_| shape())?;
                if !(1..=12).contains(&month) {
                    return Err(DateParseError::Month(month));
                }
                Ok(Self::YearMonth(year, month))
            }
        }
    }
}

impl fmt::Display for ImpreciseDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Year(year) => write!(f, "{year}"),
            Self::YearMonth(year, month) => write!(f, "{year}-{month:02}"),
        }
    }
}

/// When a role ended, or that it has not.
///
/// Wire form is `"current"` or an [`ImpreciseDate`] string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleEnd {
    Current,
    Ended(ImpreciseDate),
}

impl FromStr for RoleEnd {
    type Err = DateParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("current") {
            Ok(Self::Current)
        } else {
            s.parse().map(Self::Ended)
        }
    }
}

impl fmt::Display for RoleEnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Current => f.write_str("current"),
            Self::Ended(date) => date.fmt(f),
        }
    }
}

// Hand-written serde impls rather than derives: the wire form is a single
// string, which no derive combination produces. An untagged enum over
// `Year(u16)` would expect a JSON number, and the content file holds `"2021"`.
macro_rules! serde_via_string {
    ($ty:ty) => {
        impl Serialize for $ty {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(deserializer)?;
                raw.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

serde_via_string!(ImpreciseDate);
serde_via_string!(RoleEnd);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_precisions() {
        assert_eq!("2006".parse(), Ok(ImpreciseDate::Year(2006)));
        assert_eq!("2023-10".parse(), Ok(ImpreciseDate::YearMonth(2023, 10)));
    }

    #[test]
    fn preserves_precision_on_round_trip() {
        for input in ["2006", "2023-10", "2023-01"] {
            let parsed: ImpreciseDate = input.parse().unwrap();
            assert_eq!(parsed.to_string(), input);
        }
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(
            "20x6".parse::<ImpreciseDate>(),
            Err(DateParseError::Shape("20x6".to_owned()))
        );
        assert_eq!(
            "2023-13".parse::<ImpreciseDate>(),
            Err(DateParseError::Month(13))
        );
    }

    #[test]
    fn orders_across_precisions() {
        let mut dates = [
            ImpreciseDate::YearMonth(2023, 10),
            ImpreciseDate::Year(2006),
            ImpreciseDate::YearMonth(2021, 3),
            ImpreciseDate::Year(2016),
        ];
        dates.sort();
        assert_eq!(
            dates,
            [
                ImpreciseDate::Year(2006),
                ImpreciseDate::Year(2016),
                ImpreciseDate::YearMonth(2021, 3),
                ImpreciseDate::YearMonth(2023, 10),
            ]
        );
    }

    #[test]
    fn role_end_handles_current() {
        assert_eq!("current".parse(), Ok(RoleEnd::Current));
        assert_eq!("Current".parse(), Ok(RoleEnd::Current));
        assert_eq!(RoleEnd::Current.to_string(), "current");
    }

    #[test]
    fn serializes_as_a_plain_string() {
        let json = serde_json::to_string(&RoleEnd::Ended(ImpreciseDate::YearMonth(2023, 9)));
        assert_eq!(json.unwrap(), r#""2023-09""#);

        let parsed: RoleEnd = serde_json::from_str(r#""current""#).unwrap();
        assert_eq!(parsed, RoleEnd::Current);
    }
}
