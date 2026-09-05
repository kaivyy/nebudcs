//! Schema versioning for project files and equipment definitions.
//!
//! [`SchemaVersion`] follows semantic versioning (MAJOR.MINOR.PATCH) and is
//! used to track the format version of persisted artefacts so that migrations
//! can be applied correctly when loading older files.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{NebuError, NebuResult};

/// A semantic version triple (MAJOR, MINOR, PATCH) for NebuDCS artefacts.
///
/// Ordering follows semver conventions:
/// - MAJOR differences are breaking.
/// - MINOR differences add features in a backward-compatible way.
/// - PATCH differences are backward-compatible bug-fixes.
///
/// # Serialization
///
/// Serialises as the compact string `"MAJOR.MINOR.PATCH"` (e.g. `"0.1.0"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SchemaVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SchemaVersion {
    /// The version used by this release of NebuDCS for new project files.
    pub const CURRENT: SchemaVersion = SchemaVersion {
        major: 0,
        minor: 1,
        patch: 0,
    };

    /// Construct a version directly from components.
    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Returns `true` if `self` is backward-compatible with `other`.
    ///
    /// Same MAJOR and `self` MINOR ≥ `other` MINOR.
    #[must_use]
    pub fn is_compatible_with(self, other: SchemaVersion) -> bool {
        self.major == other.major && self.minor >= other.minor
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl From<SchemaVersion> for String {
    fn from(v: SchemaVersion) -> String {
        v.to_string()
    }
}

impl TryFrom<String> for SchemaVersion {
    type Error = NebuError;
    fn try_from(s: String) -> NebuResult<Self> {
        s.parse()
    }
}

impl FromStr for SchemaVersion {
    type Err = NebuError;

    fn from_str(s: &str) -> NebuResult<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(NebuError::InvalidSchemaVersion {
                input: s.to_owned(),
            });
        }
        let parse_part = |p: &str| -> NebuResult<u32> {
            p.parse::<u32>()
                .map_err(|_| NebuError::InvalidSchemaVersion {
                    input: s.to_owned(),
                })
        };
        Ok(Self {
            major: parse_part(parts[0])?,
            minor: parse_part(parts[1])?,
            patch: parse_part(parts[2])?,
        })
    }
}

impl PartialOrd for SchemaVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SchemaVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_version_parse() {
        let v: SchemaVersion = "1.2.3".parse().expect("valid version");
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_schema_version_display() {
        let v = SchemaVersion::new(0, 1, 0);
        assert_eq!(v.to_string(), "0.1.0");
    }

    #[test]
    fn test_schema_version_ordering() {
        let v100: SchemaVersion = "1.0.0".parse().unwrap();
        let v110: SchemaVersion = "1.1.0".parse().unwrap();
        let v200: SchemaVersion = "2.0.0".parse().unwrap();
        assert!(v100 < v110);
        assert!(v110 < v200);
        assert!(v100 < v200);
        assert_eq!(v100, v100);
    }

    #[test]
    fn test_schema_version_compatibility() {
        let v010: SchemaVersion = "0.1.0".parse().unwrap();
        let v011: SchemaVersion = "0.1.1".parse().unwrap();
        let v020: SchemaVersion = "0.2.0".parse().unwrap();
        let v100: SchemaVersion = "1.0.0".parse().unwrap();

        // v011 is compatible with v010 (same major, same minor, higher patch)
        assert!(v011.is_compatible_with(v010));
        // v020 is compatible with v010 (same major, higher minor)
        assert!(v020.is_compatible_with(v010));
        // v010 is NOT compatible with v020 (older minor)
        assert!(!v010.is_compatible_with(v020));
        // v100 is NOT compatible with v010 (different major)
        assert!(!v100.is_compatible_with(v010));
    }

    #[test]
    fn test_schema_version_rejects_invalid() {
        assert!("not.a.version".parse::<SchemaVersion>().is_err());
        assert!("1.2".parse::<SchemaVersion>().is_err());
        assert!("1.2.3.4".parse::<SchemaVersion>().is_err());
        assert!("a.b.c".parse::<SchemaVersion>().is_err());
    }

    #[test]
    fn test_schema_version_serde_round_trip() {
        let v = SchemaVersion::new(1, 2, 3);
        let json = serde_json::to_string(&v).expect("serialize");
        let decoded: SchemaVersion = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(v, decoded);
    }
}
