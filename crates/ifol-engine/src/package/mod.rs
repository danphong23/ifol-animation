//! Package identity, versioning, manifest, dependency resolution, and lock.
//!
//! This module implements stable package identification, semantic versioning
//! with constraint matching, package manifest declarations, deterministic
//! dependency resolution with cycle detection, and the lock result format.

mod manifest;
mod resolution;
mod resolver;
mod version;

pub use manifest::{
    EnginePackage, PackageDependency, PackageError, PackageManifest, PackageRegistration,
};
pub use resolution::{PackageLock, ResolveError, ResolvedPackage};
pub use resolver::PackageResolver;
pub use version::{Version, VersionReq};

use std::fmt;

/// A stable, opaque package identifier.
///
/// `PackageId` does not depend on crate name, file path, or load address.
/// It is a semantic identity chosen by the package author and must remain
/// stable across releases.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageId(String);

impl PackageId {
    /// Creates a new `PackageId`.
    ///
    /// # Errors
    ///
    /// Returns `None` if the id is empty or contains invalid characters.
    /// Valid characters: ASCII alphanumeric, `-`, `_`, `.`.
    pub fn new(id: impl Into<String>) -> Option<Self> {
        let id = id.into();
        if id.is_empty() {
            return None;
        }
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return None;
        }
        Some(Self(id))
    }

    /// Returns the string representation.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_package_ids() {
        assert!(PackageId::new("my-package").is_some());
        assert!(PackageId::new("pkg_alpha").is_some());
        assert!(PackageId::new("com.ifol.render").is_some());
        assert!(PackageId::new("a123").is_some());
    }

    #[test]
    fn invalid_package_ids() {
        assert!(PackageId::new("").is_none());
        assert!(PackageId::new("has space").is_none());
        assert!(PackageId::new("path/sep").is_none());
        assert!(PackageId::new("special@char").is_none());
    }

    #[test]
    fn package_id_equality() {
        let a = PackageId::new("alpha").unwrap();
        let b = PackageId::new("alpha").unwrap();
        let c = PackageId::new("beta").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn package_id_display() {
        let id = PackageId::new("my-pkg").unwrap();
        assert_eq!(format!("{id}"), "my-pkg");
    }

    #[test]
    fn package_id_ordering_is_deterministic() {
        let mut ids: Vec<PackageId> = vec!["beta", "alpha", "gamma"]
            .into_iter()
            .map(|s| PackageId::new(s).unwrap())
            .collect();
        ids.sort();
        assert_eq!(ids[0].as_str(), "alpha");
        assert_eq!(ids[1].as_str(), "beta");
        assert_eq!(ids[2].as_str(), "gamma");
    }
}
