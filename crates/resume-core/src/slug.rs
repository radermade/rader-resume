//! Stable identifiers for content entries.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SlugError {
    #[error("slug is empty")]
    Empty,
    #[error("slug `{0}` must be lowercase alphanumeric with internal hyphens")]
    Invalid(String),
}

/// A URL-safe, stable identifier.
///
/// A newtype rather than a `String` because slugs end up in routes, cache
/// keys, and model-generated citations. Validating once at parse time means
/// nothing downstream has to wonder.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Slug(String);

impl Slug {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Slug {
    type Err = SlugError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(SlugError::Empty);
        }

        let shape_ok = s
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        let edges_ok = !s.starts_with('-') && !s.ends_with('-');

        if shape_ok && edges_ok && !s.contains("--") {
            Ok(Self(s.to_owned()))
        } else {
            Err(SlugError::Invalid(s.to_owned()))
        }
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for Slug {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Slug {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_slugs() {
        for input in ["nipr", "walt-disney-company", "faa-part-107"] {
            assert_eq!(input.parse::<Slug>().unwrap().as_str(), input);
        }
    }

    #[test]
    fn rejects_invalid_slugs() {
        assert_eq!("".parse::<Slug>(), Err(SlugError::Empty));
        for input in [
            "Cerner",
            "has space",
            "-leading",
            "trailing-",
            "double--hyphen",
        ] {
            assert!(
                matches!(input.parse::<Slug>(), Err(SlugError::Invalid(_))),
                "{input} should be rejected"
            );
        }
    }
}
