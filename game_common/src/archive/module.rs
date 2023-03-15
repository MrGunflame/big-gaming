use std::fmt::{self, Display, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Module {
    pub id: Uuid,
    pub name: String,
    pub version: Version,
    pub description: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}

/// A semver version string.
///
/// See <https://semver.org/>
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {}

impl FromStr for Version {
    type Err = ParseVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.');

        let major = parts.next().ok_or(VersionErrorKind::MissingSegment)?;
        let minor = parts.next().ok_or(VersionErrorKind::MissingSegment)?;
        let patch = parts.next().ok_or(VersionErrorKind::MissingSegment)?;

        if parts.next().is_some() {
            return Err(VersionErrorKind::Trailing.into());
        }

        let major = major.parse().map_err(VersionErrorKind::ParseIntError)?;
        let minor = minor.parse().map_err(VersionErrorKind::ParseIntError)?;
        let patch = patch.parse().map_err(VersionErrorKind::ParseIntError)?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(VersionVisitor)
    }
}

struct VersionVisitor;

impl<'de> Visitor<'de> for VersionVisitor {
    type Value = Version;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a semver version string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse().map_err(E::custom)
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&v)
    }
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct ParseVersionError(#[from] VersionErrorKind);

#[derive(Clone, Debug, Error)]
enum VersionErrorKind {
    #[error("failed to parse int: {0}")]
    ParseIntError(#[from] ParseIntError),
    #[error("missing semver segment")]
    MissingSegment,
    #[error("trailing")]
    Trailing,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dependency {
    pub id: Uuid,
    pub name: Option<String>,
    pub version: Version,
}

#[cfg(test)]
mod tests {
    use super::Version;

    #[test]
    fn test_version_parse() {
        let version: Version = "0.1.0".parse().unwrap();
        assert_eq!(
            version,
            Version {
                major: 0,
                minor: 1,
                patch: 0,
            }
        );

        assert!("".parse::<Version>().is_err());
        assert!("1.0".parse::<Version>().is_err());
        assert!("1.0.".parse::<Version>().is_err());
        assert!("1.0.0.".parse::<Version>().is_err());
        assert!("1.0.0.0".parse::<Version>().is_err());
        assert!("1.-1.0".parse::<Version>().is_err());
        assert!("1.a.0".parse::<Version>().is_err());
    }
}
