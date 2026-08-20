//! Runtime configuration for [`EngineBuilder`](crate::EngineBuilder).
//!
//! `EngineConfig` is deliberately in-memory. It describes what the host wants
//! to activate; it does not know how a project is stored, serialized, or
//! discovered. A higher-level project/package tool may translate its own
//! manifest into this value.

use crate::namespace::NamespaceRegistry;
use crate::package::{PackageDependency, PackageLock};

/// Immutable inputs required to construct an engine runtime.
#[derive(Debug, Clone, Default)]
pub struct EngineConfig {
    required_packages: Vec<PackageDependency>,
    expected_lock: Option<PackageLock>,
    namespaces: NamespaceRegistry,
}

impl EngineConfig {
    /// Creates an empty configuration. An empty configuration is valid and
    /// produces an engine with no package contributions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a required package root. Only its transitive dependency closure is
    /// activated by the builder.
    pub fn require_package(mut self, package: PackageDependency) -> Self {
        self.required_packages.push(package);
        self
    }

    /// Requires the resolved package graph to equal `lock` exactly.
    ///
    /// This is a runtime reproducibility check, not a lock-file parser. File
    /// formats belong to an outer project layer.
    pub fn with_expected_lock(mut self, lock: PackageLock) -> Self {
        self.expected_lock = Some(lock);
        self
    }

    /// Supplies previously claimed package namespaces.
    pub fn with_namespaces(mut self, namespaces: NamespaceRegistry) -> Self {
        self.namespaces = namespaces;
        self
    }

    /// Returns the number of explicitly required package roots.
    pub fn required_package_count(&self) -> usize {
        self.required_packages.len()
    }

    pub(crate) fn required_packages(&self) -> &[PackageDependency] {
        &self.required_packages
    }

    pub(crate) fn expected_lock(&self) -> Option<&PackageLock> {
        self.expected_lock.as_ref()
    }

    pub(crate) fn namespaces(&self) -> NamespaceRegistry {
        self.namespaces.clone()
    }
}
