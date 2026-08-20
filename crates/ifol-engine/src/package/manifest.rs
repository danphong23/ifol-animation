//! Package manifest — the declaration of a package's identity,
//! version, dependencies, and claimed namespaces.

use super::PackageId;
use super::version::{Version, VersionReq};
use crate::registration::RegistrationContext;
use std::sync::Mutex;
use thiserror::Error;

/// Error returned when a package cannot prepare its contribution.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PackageError {
    #[error("package registration failed: {0}")]
    Registration(String),

    #[error("package registration was already consumed")]
    AlreadyRegistered,
}

/// The package contract consumed by `ifol-engine`.
///
/// A package owns its semantics and contributes only through the controlled
/// [`RegistrationContext`]. The engine uses the manifest for resolution and
/// invokes `register` only after the complete package set is resolved.
pub trait EnginePackage: Send + Sync {
    /// Returns the stable manifest used for dependency resolution.
    fn manifest(&self) -> &PackageManifest;

    /// Stages this package's ECS/project contribution.
    fn register(&self, context: &mut RegistrationContext) -> Result<(), PackageError>;
}

/// Programmatic package adapter for static registration and tests.
///
/// This is an explicit package object, not a hidden engine feature. Production
/// packages may implement [`EnginePackage`] directly when they need richer
/// ownership or lifecycle behavior.
pub struct PackageRegistration<F> {
    manifest: PackageManifest,
    register_fn: Mutex<Option<F>>,
}

impl<F> PackageRegistration<F> {
    /// Creates a package from a manifest and a registration function.
    pub fn new(manifest: PackageManifest, register_fn: F) -> Self {
        Self {
            manifest,
            register_fn: Mutex::new(Some(register_fn)),
        }
    }
}

impl<F> EnginePackage for PackageRegistration<F>
where
    F: FnOnce(&mut RegistrationContext) + Send,
{
    fn manifest(&self) -> &PackageManifest {
        &self.manifest
    }

    fn register(&self, context: &mut RegistrationContext) -> Result<(), PackageError> {
        let register_fn = self
            .register_fn
            .lock()
            .map_err(|_| PackageError::Registration("registration lock poisoned".into()))?
            .take()
            .ok_or(PackageError::AlreadyRegistered)?;
        register_fn(context);
        Ok(())
    }
}

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
