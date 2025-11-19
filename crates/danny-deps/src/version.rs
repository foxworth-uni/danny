//! Version parsing and comparison for different ecosystems

use crate::{Ecosystem, Error, Result};
use semver::{Version, VersionReq as SemverReq};
use std::cmp::Ordering;

/// Parsed version requirement that handles both semver and npm-style
#[derive(Debug, Clone)]
pub enum ParsedVersionReq {
    /// Semver requirement (Cargo: "1.0", "^1.0", ">=1.0, <2.0")
    Semver(SemverReq),
    /// npm-style requirement (npm: "^1.0.0", "~1.0.0", "*", "latest")
    Npm(node_semver::Range),
}

impl ParsedVersionReq {
    /// Parse a version requirement based on ecosystem
    pub fn parse(raw: &str, ecosystem: Ecosystem) -> Result<Self> {
        match ecosystem {
            Ecosystem::Rust => {
                let req = SemverReq::parse(raw)
                    .map_err(|e| Error::InvalidVersion(raw.to_string(), e.to_string()))?;
                Ok(Self::Semver(req))
            }
            Ecosystem::JavaScript => {
                let range = node_semver::Range::parse(raw)
                    .map_err(|e| Error::InvalidVersion(raw.to_string(), e.to_string()))?;
                Ok(Self::Npm(range))
            }
        }
    }

    /// Check if a version satisfies this requirement
    pub fn matches(&self, version: &str) -> Result<bool> {
        match self {
            Self::Semver(req) => {
                let v = Version::parse(version)
                    .map_err(|e| Error::InvalidVersion(version.to_string(), e.to_string()))?;
                Ok(req.matches(&v))
            }
            Self::Npm(range) => {
                let v = node_semver::Version::parse(version)
                    .map_err(|e| Error::InvalidVersion(version.to_string(), e.to_string()))?;
                Ok(range.satisfies(&v))
            }
        }
    }
}

/// Compare two versions
pub fn compare_versions(a: &str, b: &str, ecosystem: Ecosystem) -> Result<Ordering> {
    match ecosystem {
        Ecosystem::Rust => {
            let va = Version::parse(a)
                .map_err(|e| Error::InvalidVersion(a.to_string(), e.to_string()))?;
            let vb = Version::parse(b)
                .map_err(|e| Error::InvalidVersion(b.to_string(), e.to_string()))?;
            Ok(va.cmp(&vb))
        }
        Ecosystem::JavaScript => {
            let va = node_semver::Version::parse(a)
                .map_err(|e| Error::InvalidVersion(a.to_string(), e.to_string()))?;
            let vb = node_semver::Version::parse(b)
                .map_err(|e| Error::InvalidVersion(b.to_string(), e.to_string()))?;
            Ok(va.cmp(&vb))
        }
    }
}

/// Determine update type (major, minor, patch)
pub fn update_type(current: &str, latest: &str, ecosystem: Ecosystem) -> Result<UpdateType> {
    match ecosystem {
        Ecosystem::Rust => {
            let c = Version::parse(current)
                .map_err(|e| Error::InvalidVersion(current.to_string(), e.to_string()))?;
            let l = Version::parse(latest)
                .map_err(|e| Error::InvalidVersion(latest.to_string(), e.to_string()))?;

            Ok(if l.major > c.major {
                UpdateType::Major
            } else if l.minor > c.minor {
                UpdateType::Minor
            } else if l.patch > c.patch {
                UpdateType::Patch
            } else {
                UpdateType::None
            })
        }
        Ecosystem::JavaScript => {
            let c = node_semver::Version::parse(current)
                .map_err(|e| Error::InvalidVersion(current.to_string(), e.to_string()))?;
            let l = node_semver::Version::parse(latest)
                .map_err(|e| Error::InvalidVersion(latest.to_string(), e.to_string()))?;

            Ok(if l.major > c.major {
                UpdateType::Major
            } else if l.minor > c.minor {
                UpdateType::Minor
            } else if l.patch > c.patch {
                UpdateType::Patch
            } else {
                UpdateType::None
            })
        }
    }
}

/// Type of version update
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateType {
    /// Major version bump (1.0.0 -> 2.0.0) - potentially breaking
    Major,
    /// Minor version bump (1.0.0 -> 1.1.0) - new features
    Minor,
    /// Patch version bump (1.0.0 -> 1.0.1) - bug fixes
    Patch,
    /// No update needed - already on latest
    None,
}

impl UpdateType {
    /// Check if this is a breaking change
    pub fn is_breaking(&self) -> bool {
        matches!(self, UpdateType::Major)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_parsing() {
        let req = ParsedVersionReq::parse("1.0", Ecosystem::Rust).unwrap();
        assert!(matches!(req, ParsedVersionReq::Semver(_)));

        let req = ParsedVersionReq::parse("^1.0", Ecosystem::Rust).unwrap();
        assert!(matches!(req, ParsedVersionReq::Semver(_)));

        let req = ParsedVersionReq::parse(">=1.0, <2.0", Ecosystem::Rust).unwrap();
        assert!(matches!(req, ParsedVersionReq::Semver(_)));
    }

    #[test]
    fn test_npm_parsing() {
        let req = ParsedVersionReq::parse("^1.0.0", Ecosystem::JavaScript).unwrap();
        assert!(matches!(req, ParsedVersionReq::Npm(_)));

        let req = ParsedVersionReq::parse("~1.0.0", Ecosystem::JavaScript).unwrap();
        assert!(matches!(req, ParsedVersionReq::Npm(_)));

        let req = ParsedVersionReq::parse("*", Ecosystem::JavaScript).unwrap();
        assert!(matches!(req, ParsedVersionReq::Npm(_)));
    }

    #[test]
    fn test_version_comparison() {
        assert_eq!(
            compare_versions("1.0.0", "1.0.1", Ecosystem::Rust).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            compare_versions("1.0.0", "1.0.0", Ecosystem::Rust).unwrap(),
            Ordering::Equal
        );
        assert_eq!(
            compare_versions("2.0.0", "1.0.0", Ecosystem::Rust).unwrap(),
            Ordering::Greater
        );
    }

    #[test]
    fn test_update_type() {
        assert_eq!(
            update_type("1.0.0", "2.0.0", Ecosystem::Rust).unwrap(),
            UpdateType::Major
        );
        assert_eq!(
            update_type("1.0.0", "1.1.0", Ecosystem::Rust).unwrap(),
            UpdateType::Minor
        );
        assert_eq!(
            update_type("1.0.0", "1.0.1", Ecosystem::Rust).unwrap(),
            UpdateType::Patch
        );
        assert_eq!(
            update_type("1.0.0", "1.0.0", Ecosystem::Rust).unwrap(),
            UpdateType::None
        );
    }

    #[test]
    fn test_matches() {
        let req = ParsedVersionReq::parse("^1.0.0", Ecosystem::JavaScript).unwrap();
        assert!(req.matches("1.0.0").unwrap());
        assert!(req.matches("1.0.1").unwrap());
        assert!(req.matches("1.1.0").unwrap());
        assert!(!req.matches("2.0.0").unwrap());
    }
}

#[cfg(test)]
#[cfg(feature = "property-tests")]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    /// Property: Version comparison is transitive
    /// If a < b and b < c, then a < c
    proptest! {
        #[test]
        fn version_comparison_is_transitive(
            a in r"[0-9]+\.[0-9]+\.[0-9]+",
            b in r"[0-9]+\.[0-9]+\.[0-9]+",
            c in r"[0-9]+\.[0-9]+\.[0-9]+"
        ) {
            let ordering_ab = compare_versions(&a, &b, Ecosystem::Rust).ok();
            let ordering_bc = compare_versions(&b, &c, Ecosystem::Rust).ok();
            let ordering_ac = compare_versions(&a, &c, Ecosystem::Rust).ok();

            if let (Some(ab), Some(bc), Some(ac)) = (ordering_ab, ordering_bc, ordering_ac) {
                if ab == Ordering::Less && bc == Ordering::Less {
                    prop_assert_eq!(ac, Ordering::Less);
                }
            }
        }

        #[test]
        fn version_comparison_is_reflexive(
            version in r"[0-9]+\.[0-9]+\.[0-9]+"
        ) {
            let ordering = compare_versions(&version, &version, Ecosystem::Rust).unwrap();
            prop_assert_eq!(ordering, Ordering::Equal);
        }

        #[test]
        fn update_type_is_consistent(
            current in r"[0-9]+\.[0-9]+\.[0-9]+",
            latest in r"[0-9]+\.[0-9]+\.[0-9]+"
        ) {
            let ut = update_type(&current, &latest, Ecosystem::Rust).ok();
            let cmp = compare_versions(&current, &latest, Ecosystem::Rust).ok();

            if let (Some(update_t), Some(ordering)) = (ut, cmp) {
                match update_t {
                    UpdateType::Major => prop_assert_eq!(ordering, Ordering::Less),
                    UpdateType::Minor => prop_assert_eq!(ordering, Ordering::Less),
                    UpdateType::Patch => prop_assert_eq!(ordering, Ordering::Less),
                    UpdateType::None => prop_assert!(ordering == Ordering::Equal || ordering == Ordering::Greater),
                }
            }
        }
    }
}
