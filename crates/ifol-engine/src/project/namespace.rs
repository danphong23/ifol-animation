//! Hierarchical package namespace system.
//!
//! Enforces namespace boundaries between packages. No two packages
//! can claim the same or conflicting hierarchical namespaces.

use crate::package::PackageId;
use std::collections::BTreeMap;
use std::fmt;
use thiserror::Error;

/// Hierarchical namespace identifier (e.g. `"core.render"` or `"vendor.physics"`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Namespace(String);

impl Namespace {
    /// Creates and validates a namespace string.
    pub fn new(s: impl Into<String>) -> Option<Self> {
        let s = s.into();
        if s.is_empty() {
            return None;
        }

        // Each segment separated by '.' must be non-empty and valid identifier
        for segment in s.split('.') {
            if segment.is_empty() {
                return None;
            }
            if !segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                return None;
            }
        }

        Some(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Errors occurring during namespace registration.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum NamespaceError {
    #[error("namespace '{namespace}' is already claimed by package '{owner}'")]
    DuplicateClaim { namespace: String, owner: PackageId },

    #[error(
        "namespace conflict: '{candidate}' conflicts with existing prefix claim '{existing}' by '{owner}'"
    )]
    PrefixConflict {
        candidate: String,
        existing: String,
        owner: PackageId,
    },
}

/// Registry managing claimed package namespaces.
#[derive(Debug, Default, Clone)]
pub struct NamespaceRegistry {
    claims: BTreeMap<Namespace, PackageId>,
}

impl NamespaceRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self {
            claims: BTreeMap::new(),
        }
    }

    /// Claims a namespace for a package.
    ///
    /// Checks for exact duplicates and hierarchical prefix conflicts.
    pub fn claim(&mut self, owner: PackageId, ns: Namespace) -> Result<(), NamespaceError> {
        // Check exact match
        if let Some(existing_owner) = self.claims.get(&ns) {
            return Err(NamespaceError::DuplicateClaim {
                namespace: ns.to_string(),
                owner: existing_owner.clone(),
            });
        }

        // Check prefix conflicts (e.g. 'core' vs 'core.render')
        for (existing_ns, existing_owner) in &self.claims {
            let e_str = existing_ns.as_str();
            let c_str = ns.as_str();

            // Candidate is a child of existing (e.g. existing 'core', candidate 'core.render')
            if c_str.starts_with(e_str) && c_str.chars().nth(e_str.len()) == Some('.') {
                return Err(NamespaceError::PrefixConflict {
                    candidate: ns.to_string(),
                    existing: existing_ns.to_string(),
                    owner: existing_owner.clone(),
                });
            }

            // Existing is a child of candidate (e.g. existing 'core.render', candidate 'core')
            if e_str.starts_with(c_str) && e_str.chars().nth(c_str.len()) == Some('.') {
                return Err(NamespaceError::PrefixConflict {
                    candidate: ns.to_string(),
                    existing: existing_ns.to_string(),
                    owner: existing_owner.clone(),
                });
            }
        }

        self.claims.insert(ns, owner);
        Ok(())
    }

    /// Resolves the owner of a namespace, if claimed.
    pub fn get_owner(&self, ns: &Namespace) -> Option<&PackageId> {
        self.claims.get(ns)
    }

    /// Returns the total number of claimed namespaces.
    pub fn len(&self) -> usize {
        self.claims.len()
    }

    /// Returns `true` if no namespaces are claimed.
    pub fn is_empty(&self) -> bool {
        self.claims.is_empty()
    }
}
