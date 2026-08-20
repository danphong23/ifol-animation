//! Semantic versioning and version constraint matching.

use std::fmt;

/// A semantic version: `major.minor.patch`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Creates a new version.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parses a version string `"X.Y.Z"`.
    ///
    /// Returns `None` if the format is invalid.
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].parse().ok()?;
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A version constraint for dependency matching.
///
/// Supports caret compatibility (default): `^X.Y.Z` means
/// `>= X.Y.Z` and `< next-breaking`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionReq {
    /// Minimum version (inclusive).
    pub min: Version,
}

impl VersionReq {
    /// Creates a caret constraint: compatible with `min` up to the next
    /// breaking change.
    pub const fn caret(min: Version) -> Self {
        Self { min }
    }

    /// Creates a constraint from a version string.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.strip_prefix('^').unwrap_or(s);
        let min = Version::parse(s)?;
        Some(Self::caret(min))
    }

    /// Returns `true` if `version` satisfies this constraint.
    ///
    /// Caret semantics:
    /// - `^0.0.Z` matches only `0.0.Z` exactly.
    /// - `^0.Y.Z` matches `0.Y.*` where `Y` is the same.
    /// - `^X.Y.Z` (X > 0) matches `X.*.*` where `X` is the same.
    pub fn matches(&self, version: &Version) -> bool {
        if version < &self.min {
            return false;
        }
        if self.min.major == 0 {
            if self.min.minor == 0 {
                // ^0.0.Z — exact patch match
                version.major == 0 && version.minor == 0 && version.patch == self.min.patch
            } else {
                // ^0.Y.Z — same minor
                version.major == 0 && version.minor == self.min.minor
            }
        } else {
            // ^X.Y.Z — same major
            version.major == self.min.major
        }
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "^{}", self.min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parse_valid() {
        assert_eq!(Version::parse("1.2.3"), Some(Version::new(1, 2, 3)));
        assert_eq!(Version::parse("0.0.0"), Some(Version::new(0, 0, 0)));
        assert_eq!(Version::parse("10.20.30"), Some(Version::new(10, 20, 30)));
    }

    #[test]
    fn version_parse_invalid() {
        assert!(Version::parse("").is_none());
        assert!(Version::parse("1.2").is_none());
        assert!(Version::parse("1.2.3.4").is_none());
        assert!(Version::parse("a.b.c").is_none());
        assert!(Version::parse("1.2.-3").is_none());
    }

    #[test]
    fn version_ordering() {
        assert!(Version::new(1, 0, 0) < Version::new(2, 0, 0));
        assert!(Version::new(1, 0, 0) < Version::new(1, 1, 0));
        assert!(Version::new(1, 0, 0) < Version::new(1, 0, 1));
        assert!(Version::new(1, 2, 3) == Version::new(1, 2, 3));
    }

    #[test]
    fn version_display() {
        assert_eq!(format!("{}", Version::new(1, 2, 3)), "1.2.3");
    }

    // Caret matching tests
    #[test]
    fn caret_major_nonzero() {
        let req = VersionReq::caret(Version::new(1, 2, 3));
        assert!(req.matches(&Version::new(1, 2, 3)));
        assert!(req.matches(&Version::new(1, 3, 0)));
        assert!(req.matches(&Version::new(1, 99, 99)));
        assert!(!req.matches(&Version::new(1, 2, 2))); // below min
        assert!(!req.matches(&Version::new(2, 0, 0))); // breaking
        assert!(!req.matches(&Version::new(0, 9, 9)));
    }

    #[test]
    fn caret_minor_nonzero() {
        let req = VersionReq::caret(Version::new(0, 2, 1));
        assert!(req.matches(&Version::new(0, 2, 1)));
        assert!(req.matches(&Version::new(0, 2, 9)));
        assert!(!req.matches(&Version::new(0, 2, 0))); // below min
        assert!(!req.matches(&Version::new(0, 3, 0))); // breaking
        assert!(!req.matches(&Version::new(1, 0, 0)));
    }

    #[test]
    fn caret_patch_only() {
        let req = VersionReq::caret(Version::new(0, 0, 5));
        assert!(req.matches(&Version::new(0, 0, 5)));
        assert!(!req.matches(&Version::new(0, 0, 6))); // exact only
        assert!(!req.matches(&Version::new(0, 0, 4)));
        assert!(!req.matches(&Version::new(0, 1, 0)));
    }

    #[test]
    fn version_req_parse() {
        let req = VersionReq::parse("^1.2.3").unwrap();
        assert_eq!(req.min, Version::new(1, 2, 3));

        // Without caret prefix
        let req2 = VersionReq::parse("1.2.3").unwrap();
        assert_eq!(req2.min, Version::new(1, 2, 3));
    }

    #[test]
    fn version_req_display() {
        let req = VersionReq::caret(Version::new(1, 2, 3));
        assert_eq!(format!("{req}"), "^1.2.3");
    }
}
