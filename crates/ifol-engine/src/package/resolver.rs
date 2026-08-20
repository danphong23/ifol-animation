//! Deterministic dependency resolver with cycle detection.
//!
//! Resolves a set of package manifests into a topologically sorted
//! activation order. The resolver is deterministic: given the same
//! set of manifests (regardless of insertion order), it produces
//! the same lock result.

use super::PackageId;
use super::manifest::PackageManifest;
use super::version::Version;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use thiserror::Error;

/// Errors produced by the package resolver.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ResolveError {
    #[error("duplicate package ID: '{0}'")]
    DuplicateId(String),

    #[error("missing dependency: package '{from}' requires '{dep}' which is not available")]
    MissingDependency { from: String, dep: String },

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
    /// The package ID.
    pub id: PackageId,
    /// The resolved version.
    pub version: Version,
    /// Resolved dependency IDs (subset of the lock).
    pub dependencies: Vec<PackageId>,
}

/// The lock result: an ordered list of resolved packages.
///
/// Packages are in topological dependency order (dependencies before
/// dependants). The order is deterministic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageLock {
    /// Packages in topological order.
    pub packages: Vec<ResolvedPackage>,
}

impl PackageLock {
    /// Returns the number of resolved packages.
    pub fn len(&self) -> usize {
        self.packages.len()
    }

    /// Returns `true` if no packages were resolved.
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    /// Finds a resolved package by ID.
    pub fn find(&self, id: &PackageId) -> Option<&ResolvedPackage> {
        self.packages.iter().find(|p| &p.id == id)
    }
}

/// Deterministic dependency resolver.
pub struct PackageResolver {
    manifests: Vec<PackageManifest>,
}

impl PackageResolver {
    /// Creates an empty resolver.
    pub fn new() -> Self {
        Self {
            manifests: Vec::new(),
        }
    }

    /// Adds a package manifest as a candidate.
    pub fn add(&mut self, manifest: PackageManifest) {
        self.manifests.push(manifest);
    }

    /// Resolves all added manifests into a lock result.
    ///
    /// # Determinism
    ///
    /// The resolver sorts candidates by `PackageId` before processing
    /// to ensure the same result regardless of insertion order.
    pub fn resolve(self) -> Result<PackageLock, ResolveError> {
        // Sort by ID for determinism
        let mut manifests = self.manifests;
        manifests.sort_by(|a, b| a.id.cmp(&b.id));

        // Index by ID, detect duplicates
        let mut index: BTreeMap<PackageId, &PackageManifest> = BTreeMap::new();
        for m in &manifests {
            if index.contains_key(&m.id) {
                return Err(ResolveError::DuplicateId(m.id.as_str().to_string()));
            }
            index.insert(m.id.clone(), m);
        }

        // Validate dependencies and version constraints
        for m in &manifests {
            for dep in &m.dependencies {
                let Some(dep_manifest) = index.get(&dep.package_id) else {
                    return Err(ResolveError::MissingDependency {
                        from: m.id.as_str().to_string(),
                        dep: dep.package_id.as_str().to_string(),
                    });
                };
                if !dep.version_req.matches(&dep_manifest.version) {
                    return Err(ResolveError::VersionConflict {
                        from: m.id.as_str().to_string(),
                        dep: dep.package_id.as_str().to_string(),
                        required: format!("{}", dep.version_req),
                        available: format!("{}", dep_manifest.version),
                    });
                }
            }
        }

        // Topological sort (Kahn's algorithm) with cycle detection
        let mut in_degree: BTreeMap<&PackageId, usize> = BTreeMap::new();
        let mut dependants: BTreeMap<&PackageId, BTreeSet<&PackageId>> = BTreeMap::new();

        for m in &manifests {
            in_degree.entry(&m.id).or_insert(0);
            for dep in &m.dependencies {
                *in_degree.entry(&m.id).or_insert(0) += 1;
                dependants.entry(&dep.package_id).or_default().insert(&m.id);
            }
        }

        let mut queue: VecDeque<&PackageId> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut sorted: Vec<ResolvedPackage> = Vec::new();

        while let Some(id) = queue.pop_front() {
            let m = index[id];
            sorted.push(ResolvedPackage {
                id: m.id.clone(),
                version: m.version.clone(),
                dependencies: m
                    .dependencies
                    .iter()
                    .map(|d| d.package_id.clone())
                    .collect(),
            });

            if let Some(deps) = dependants.get(id) {
                for dep_id in deps {
                    let deg = in_degree.get_mut(dep_id).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dep_id);
                    }
                }
            }
        }

        if sorted.len() != manifests.len() {
            // Find packages involved in cycle
            let resolved_ids: BTreeSet<_> = sorted.iter().map(|p| &p.id).collect();
            let cycle_members: Vec<String> = manifests
                .iter()
                .filter(|m| !resolved_ids.contains(&m.id))
                .map(|m| m.id.as_str().to_string())
                .collect();
            return Err(ResolveError::CycleDetected(cycle_members.join(", ")));
        }

        Ok(PackageLock { packages: sorted })
    }
}

impl Default for PackageResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::version::VersionReq;

    fn make_manifest(id: &str, ver: (u32, u32, u32)) -> PackageManifest {
        PackageManifest::new(
            PackageId::new(id).unwrap(),
            Version::new(ver.0, ver.1, ver.2),
        )
    }

    #[test]
    fn empty_resolve() {
        let resolver = PackageResolver::new();
        let lock = resolver.resolve().unwrap();
        assert!(lock.is_empty());
        assert_eq!(lock.len(), 0);
    }

    #[test]
    fn single_package() {
        let mut resolver = PackageResolver::new();
        resolver.add(make_manifest("alpha", (1, 0, 0)));
        let lock = resolver.resolve().unwrap();
        assert_eq!(lock.len(), 1);
        assert_eq!(lock.packages[0].id.as_str(), "alpha");
    }

    #[test]
    fn duplicate_id_rejected() {
        let mut resolver = PackageResolver::new();
        resolver.add(make_manifest("alpha", (1, 0, 0)));
        resolver.add(make_manifest("alpha", (2, 0, 0)));
        let err = resolver.resolve().unwrap_err();
        assert!(matches!(err, ResolveError::DuplicateId(ref s) if s == "alpha"));
    }

    #[test]
    fn missing_dependency() {
        let mut resolver = PackageResolver::new();
        let m = PackageManifest::new(PackageId::new("beta").unwrap(), Version::new(1, 0, 0))
            .with_dependency(crate::package::PackageDependency {
                package_id: PackageId::new("alpha").unwrap(),
                version_req: VersionReq::caret(Version::new(1, 0, 0)),
            });
        resolver.add(m);
        let err = resolver.resolve().unwrap_err();
        assert!(matches!(err, ResolveError::MissingDependency { .. }));
    }

    #[test]
    fn version_conflict() {
        let mut resolver = PackageResolver::new();
        resolver.add(make_manifest("alpha", (1, 0, 0)));
        let m = PackageManifest::new(PackageId::new("beta").unwrap(), Version::new(1, 0, 0))
            .with_dependency(crate::package::PackageDependency {
                package_id: PackageId::new("alpha").unwrap(),
                version_req: VersionReq::caret(Version::new(2, 0, 0)), // alpha is 1.0.0
            });
        resolver.add(m);
        let err = resolver.resolve().unwrap_err();
        assert!(matches!(err, ResolveError::VersionConflict { .. }));
    }

    #[test]
    fn self_cycle_detected() {
        let mut resolver = PackageResolver::new();
        let m = PackageManifest::new(PackageId::new("alpha").unwrap(), Version::new(1, 0, 0))
            .with_dependency(crate::package::PackageDependency {
                package_id: PackageId::new("alpha").unwrap(),
                version_req: VersionReq::caret(Version::new(1, 0, 0)),
            });
        resolver.add(m);
        let err = resolver.resolve().unwrap_err();
        assert!(matches!(err, ResolveError::CycleDetected(_)));
    }

    #[test]
    fn multi_node_cycle_detected() {
        let mut resolver = PackageResolver::new();
        // alpha -> beta -> gamma -> alpha
        let alpha = PackageManifest::new(PackageId::new("alpha").unwrap(), Version::new(1, 0, 0))
            .with_dependency(crate::package::PackageDependency {
                package_id: PackageId::new("beta").unwrap(),
                version_req: VersionReq::caret(Version::new(1, 0, 0)),
            });
        let beta = PackageManifest::new(PackageId::new("beta").unwrap(), Version::new(1, 0, 0))
            .with_dependency(crate::package::PackageDependency {
                package_id: PackageId::new("gamma").unwrap(),
                version_req: VersionReq::caret(Version::new(1, 0, 0)),
            });
        let gamma = PackageManifest::new(PackageId::new("gamma").unwrap(), Version::new(1, 0, 0))
            .with_dependency(crate::package::PackageDependency {
                package_id: PackageId::new("alpha").unwrap(),
                version_req: VersionReq::caret(Version::new(1, 0, 0)),
            });
        resolver.add(alpha);
        resolver.add(beta);
        resolver.add(gamma);
        let err = resolver.resolve().unwrap_err();
        assert!(matches!(err, ResolveError::CycleDetected(_)));
    }

    #[test]
    fn topological_order() {
        let mut resolver = PackageResolver::new();
        resolver.add(make_manifest("alpha", (1, 0, 0)));
        let beta = PackageManifest::new(PackageId::new("beta").unwrap(), Version::new(1, 0, 0))
            .with_dependency(crate::package::PackageDependency {
                package_id: PackageId::new("alpha").unwrap(),
                version_req: VersionReq::caret(Version::new(1, 0, 0)),
            });
        resolver.add(beta);
        let lock = resolver.resolve().unwrap();
        assert_eq!(lock.len(), 2);
        // alpha must come before beta
        assert_eq!(lock.packages[0].id.as_str(), "alpha");
        assert_eq!(lock.packages[1].id.as_str(), "beta");
    }

    #[test]
    fn input_order_does_not_affect_result() {
        // Add in order: beta, alpha (beta depends on alpha)
        let mut r1 = PackageResolver::new();
        let beta = PackageManifest::new(PackageId::new("beta").unwrap(), Version::new(1, 0, 0))
            .with_dependency(crate::package::PackageDependency {
                package_id: PackageId::new("alpha").unwrap(),
                version_req: VersionReq::caret(Version::new(1, 0, 0)),
            });
        r1.add(beta.clone());
        r1.add(make_manifest("alpha", (1, 0, 0)));

        // Add in order: alpha, beta
        let mut r2 = PackageResolver::new();
        r2.add(make_manifest("alpha", (1, 0, 0)));
        r2.add(beta);

        let lock1 = r1.resolve().unwrap();
        let lock2 = r2.resolve().unwrap();
        assert_eq!(lock1, lock2, "resolution must be order-independent");
    }

    #[test]
    fn diamond_dependency() {
        //   alpha
        //  /     \
        // beta  gamma
        //  \     /
        //   delta
        let mut resolver = PackageResolver::new();
        resolver.add(make_manifest("alpha", (1, 0, 0)));

        let beta =
            make_manifest("beta", (1, 0, 0)).with_dependency(crate::package::PackageDependency {
                package_id: PackageId::new("alpha").unwrap(),
                version_req: VersionReq::caret(Version::new(1, 0, 0)),
            });
        let gamma =
            make_manifest("gamma", (1, 0, 0)).with_dependency(crate::package::PackageDependency {
                package_id: PackageId::new("alpha").unwrap(),
                version_req: VersionReq::caret(Version::new(1, 0, 0)),
            });
        let delta = PackageManifest::new(PackageId::new("delta").unwrap(), Version::new(1, 0, 0))
            .with_dependency(crate::package::PackageDependency {
                package_id: PackageId::new("beta").unwrap(),
                version_req: VersionReq::caret(Version::new(1, 0, 0)),
            })
            .with_dependency(crate::package::PackageDependency {
                package_id: PackageId::new("gamma").unwrap(),
                version_req: VersionReq::caret(Version::new(1, 0, 0)),
            });
        resolver.add(beta);
        resolver.add(gamma);
        resolver.add(delta);

        let lock = resolver.resolve().unwrap();
        assert_eq!(lock.len(), 4);
        // alpha must come first, delta must come last
        assert_eq!(lock.packages[0].id.as_str(), "alpha");
        assert_eq!(lock.packages[3].id.as_str(), "delta");
    }

    #[test]
    fn lock_find_by_id() {
        let mut resolver = PackageResolver::new();
        resolver.add(make_manifest("alpha", (1, 0, 0)));
        resolver.add(make_manifest("beta", (2, 0, 0)));
        let lock = resolver.resolve().unwrap();

        let alpha = lock.find(&PackageId::new("alpha").unwrap());
        assert!(alpha.is_some());
        assert_eq!(alpha.unwrap().version, Version::new(1, 0, 0));

        let missing = lock.find(&PackageId::new("missing").unwrap());
        assert!(missing.is_none());
    }
}
