//! Package manifest — the declaration of a package's identity,
//! version, dependencies, and claimed namespaces.

use super::PackageId;
use super::version::{Version, VersionReq};

/// A dependency constraint declared by a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDependency {
    /// The ID of the required package.
    pub package_id: PackageId,
    /// The version constraint.
    pub version_req: VersionReq,
}

/// Generic minimal manifest for a package.
///
/// See `02-package-and-registration.md` for the full contract.
#[derive(Debug, Clone)]
pub struct PackageManifest {
    /// Stable identity of the package.
    pub id: PackageId,
    /// Semantic version of the package.
    pub version: Version,
    /// Dependencies on other packages.
    pub dependencies: Vec<PackageDependency>,
}

impl PackageManifest {
    /// Creates a new manifest with no dependencies.
    pub fn new(id: PackageId, version: Version) -> Self {
        Self {
            id,
            version,
            dependencies: Vec::new(),
        }
    }

    /// Adds a dependency on another package.
    pub fn with_dependency(mut self, dep: PackageDependency) -> Self {
        self.dependencies.push(dep);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_creation() {
        let manifest =
            PackageManifest::new(PackageId::new("test-pkg").unwrap(), Version::new(1, 0, 0));
        assert_eq!(manifest.id.as_str(), "test-pkg");
        assert_eq!(manifest.version, Version::new(1, 0, 0));
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn manifest_with_dependencies() {
        let manifest = PackageManifest::new(PackageId::new("beta").unwrap(), Version::new(1, 0, 0))
            .with_dependency(PackageDependency {
                package_id: PackageId::new("alpha").unwrap(),
                version_req: VersionReq::caret(Version::new(1, 0, 0)),
            });
        assert_eq!(manifest.dependencies.len(), 1);
        assert_eq!(manifest.dependencies[0].package_id.as_str(), "alpha");
    }
}
