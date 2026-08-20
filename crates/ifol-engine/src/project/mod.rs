//! Project Container and Namespace subsystem.
//!
//! Provides the self-contained `.ifol` bundle container:
//! - Manifest validation and serialization
//! - Storage abstraction with path containment security
//! - Hierarchical namespace registry and collision prevention
//! - Reproducible lock file

mod lockfile;
mod manifest;
mod namespace;
mod storage;

pub use lockfile::PackageLockFile;
pub use manifest::{CURRENT_FORMAT_VERSION, ProjectManifest};
pub use namespace::{Namespace, NamespaceError, NamespaceRegistry};
pub use storage::{MemoryStorage, PathSecurity, ProjectStorage, StorageError};

use thiserror::Error;

/// Errors produced during project operations.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ProjectError {
    #[error("manifest error: {0}")]
    Manifest(String),

    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("namespace error: {0}")]
    Namespace(#[from] NamespaceError),

    #[error("lockfile error: {0}")]
    Lockfile(String),
}

/// A complete, self-contained `.ifol` project container.
pub struct ProjectContainer {
    pub manifest: ProjectManifest,
    pub storage: Box<dyn ProjectStorage>,
    pub namespaces: NamespaceRegistry,
    pub lockfile: Option<PackageLockFile>,
}

impl ProjectContainer {
    /// Creates a new project container with in-memory storage.
    pub fn new_memory(name: impl Into<String>, entry_scene: impl Into<String>) -> Self {
        Self {
            manifest: ProjectManifest::new(name, entry_scene),
            storage: Box::new(MemoryStorage::new()),
            namespaces: NamespaceRegistry::new(),
            lockfile: None,
        }
    }

    /// Saves the manifest and lockfile (if present) to the underlying storage.
    pub fn save(&mut self) -> Result<(), ProjectError> {
        self.manifest.validate().map_err(ProjectError::Manifest)?;

        // Write manifest.ifol
        let manifest_bytes = self.manifest.serialize().into_bytes();
        self.storage
            .write_file("manifest.ifol", &manifest_bytes)
            .map_err(ProjectError::Storage)?;

        // Write package.lock if present
        if let Some(lock) = &self.lockfile {
            let lock_bytes = lock.serialize().into_bytes();
            self.storage
                .write_file("package.lock", &lock_bytes)
                .map_err(ProjectError::Storage)?;
        }

        Ok(())
    }

    /// Loads a project container from a given storage backend.
    pub fn load(storage: Box<dyn ProjectStorage>) -> Result<Self, ProjectError> {
        // 1. Read manifest.ifol
        let manifest_bytes = storage
            .read_file("manifest.ifol")
            .map_err(ProjectError::Storage)?;
        let manifest_str =
            String::from_utf8(manifest_bytes).map_err(|e| ProjectError::Manifest(e.to_string()))?;
        let manifest = ProjectManifest::parse(&manifest_str).map_err(ProjectError::Manifest)?;

        // 2. Read package.lock if present
        let lockfile = if storage.exists("package.lock") {
            let lock_bytes = storage
                .read_file("package.lock")
                .map_err(ProjectError::Storage)?;
            let lock_str =
                String::from_utf8(lock_bytes).map_err(|e| ProjectError::Lockfile(e.to_string()))?;
            Some(PackageLockFile::parse(&lock_str).map_err(ProjectError::Lockfile)?)
        } else {
            None
        };

        Ok(Self {
            manifest,
            storage,
            namespaces: NamespaceRegistry::new(),
            lockfile,
        })
    }
}
