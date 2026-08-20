//! Runtime namespace ownership registry.
//!
//! Namespace claims are part of package composition, not project persistence.

use crate::package::PackageId;
use std::collections::BTreeMap;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Namespace(String);

impl Namespace {
    pub fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        if value.is_empty()
            || value.split('.').any(|segment| {
                segment.is_empty()
                    || !segment.chars().all(|character| {
                        character.is_ascii_alphanumeric() || character == '_' || character == '-'
                    })
            })
        {
            return None;
        }
        Some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

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

#[derive(Debug, Default, Clone)]
pub struct NamespaceRegistry {
    claims: BTreeMap<Namespace, PackageId>,
}

impl NamespaceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn claim(&mut self, owner: PackageId, namespace: Namespace) -> Result<(), NamespaceError> {
        if let Some(existing_owner) = self.claims.get(&namespace) {
            return Err(NamespaceError::DuplicateClaim {
                namespace: namespace.to_string(),
                owner: existing_owner.clone(),
            });
        }
        for (existing, existing_owner) in &self.claims {
            let candidate = namespace.as_str();
            let existing = existing.as_str();
            if (candidate.starts_with(existing)
                && candidate.as_bytes().get(existing.len()) == Some(&b'.'))
                || (existing.starts_with(candidate)
                    && existing.as_bytes().get(candidate.len()) == Some(&b'.'))
            {
                return Err(NamespaceError::PrefixConflict {
                    candidate: namespace.to_string(),
                    existing: existing.to_string(),
                    owner: existing_owner.clone(),
                });
            }
        }
        self.claims.insert(namespace, owner);
        Ok(())
    }

    pub fn get_owner(&self, namespace: &Namespace) -> Option<&PackageId> {
        self.claims.get(namespace)
    }
    pub fn len(&self) -> usize {
        self.claims.len()
    }
    pub fn is_empty(&self) -> bool {
        self.claims.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(value: &str) -> PackageId {
        PackageId::new(value).unwrap()
    }

    #[test]
    fn namespace_validation_rejects_empty_and_invalid_segments() {
        assert!(Namespace::new("").is_none());
        assert!(Namespace::new("a..b").is_none());
        assert!(Namespace::new("a/b").is_none());
        assert!(Namespace::new("a b").is_none());
        assert!(Namespace::new("core.render_2").is_some());
    }

    #[test]
    fn namespace_registry_rejects_duplicate_and_prefix_claims() {
        let mut registry = NamespaceRegistry::new();
        registry
            .claim(package("one"), Namespace::new("core").unwrap())
            .unwrap();
        let duplicate = registry.claim(package("two"), Namespace::new("core").unwrap());
        assert!(matches!(
            duplicate,
            Err(NamespaceError::DuplicateClaim { .. })
        ));
        let child = registry.claim(package("two"), Namespace::new("core.render").unwrap());
        assert!(matches!(child, Err(NamespaceError::PrefixConflict { .. })));
    }

    #[test]
    fn namespace_registry_rejects_parent_after_child_and_exposes_owner() {
        let mut registry = NamespaceRegistry::new();
        let namespace = Namespace::new("core.render").unwrap();
        registry
            .claim(package("renderer"), namespace.clone())
            .unwrap();
        let parent = registry.claim(package("core"), Namespace::new("core").unwrap());
        assert!(matches!(parent, Err(NamespaceError::PrefixConflict { .. })));
        assert_eq!(registry.get_owner(&namespace).unwrap().as_str(), "renderer");
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }
}
