//! Dynamic reconfiguration plan.
//!
//! Describes changes to the active package set and resource providers:
//! - Packages to add or activate
//! - Packages to remove or deactivate
//! - New package manifests for dependency re-resolution

use crate::package::{PackageId, PackageManifest};
use std::collections::BTreeSet;

/// Delta plan describing changes to be applied to a running engine.
#[derive(Debug, Clone, Default)]
pub struct ReconfigurationPlan {
    /// Packages to add or update.
    pub packages_to_add: Vec<PackageManifest>,
    /// Packages to remove.
    pub packages_to_remove: BTreeSet<PackageId>,
}

impl ReconfigurationPlan {
    /// Creates an empty reconfiguration plan.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a package manifest to be added or updated.
    pub fn add_package(mut self, manifest: PackageManifest) -> Self {
        self.packages_to_add.push(manifest);
        self
    }

    /// Marks a package for removal.
    pub fn remove_package(mut self, id: PackageId) -> Self {
        self.packages_to_remove.insert(id);
        self
    }

    /// Returns `true` if this plan contains no modifications.
    pub fn is_empty(&self) -> bool {
        self.packages_to_add.is_empty() && self.packages_to_remove.is_empty()
    }
}
