use super::PackageId;
use super::version::Version;
use thiserror::Error;

/// Errors produced by the package resolver.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ResolveError {
    #[error("duplicate package ID: '{0}'")]
    DuplicateId(String),
    #[error("missing dependency: package '{from}' requires '{dep}' which is not available")]
    MissingDependency { from: String, dep: String },
    #[error("required root package '{0}' is not available")]
    MissingRoot(String),
    #[error(
        "version conflict: package '{from}' requires '{dep}' {required}, but available is {available}"
    )]
    VersionConflict {
        from: String,
        dep: String,
        required: String,
        available: String,
    },
    #[error("dependency cycle detected involving: {0}")]
    CycleDetected(String),
}

/// A successfully resolved package in the lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPackage {
    pub id: PackageId,
    pub version: Version,
    pub dependencies: Vec<PackageId>,
}

/// Deterministic topological package lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageLock {
    pub packages: Vec<ResolvedPackage>,
}

impl PackageLock {
    pub fn len(&self) -> usize {
        self.packages.len()
    }
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
    pub fn find(&self, id: &PackageId) -> Option<&ResolvedPackage> {
        self.packages.iter().find(|package| &package.id == id)
    }
}
