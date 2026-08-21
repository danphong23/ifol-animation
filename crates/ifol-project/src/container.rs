use crate::lockfile::PackageLockFile;
use crate::manifest::ProjectManifest;
use crate::storage::{
    MemoryStorage, PACKAGE_LOCK_PATH, PROJECT_MANIFEST_PATH, ProjectStorage, StorageError,
};
use ifol_engine::EngineConfig;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ProjectError {
    #[error("manifest error: {0}")]
    Manifest(String),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("lockfile error: {0}")]
    Lockfile(String),
}

pub struct ProjectContainer {
    pub manifest: ProjectManifest,
    pub storage: Box<dyn ProjectStorage>,
    pub lockfile: Option<PackageLockFile>,
}

impl ProjectContainer {
    pub fn new_memory(name: impl Into<String>, entry_scene: impl Into<String>) -> Self {
        Self {
            manifest: ProjectManifest::new(name, entry_scene),
            storage: Box::new(MemoryStorage::new()),
            lockfile: None,
        }
    }
    pub fn save(&mut self) -> Result<(), ProjectError> {
        self.manifest.validate().map_err(ProjectError::Manifest)?;
        let mut files = vec![(
            PROJECT_MANIFEST_PATH,
            self.manifest.serialize().into_bytes(),
        )];
        if let Some(lock) = &self.lockfile {
            files.push((PACKAGE_LOCK_PATH, lock.serialize().into_bytes()));
        }
        self.storage
            .write_files(&files)
            .map_err(ProjectError::Storage)
    }
    pub fn load(storage: Box<dyn ProjectStorage>) -> Result<Self, ProjectError> {
        let manifest_bytes = storage
            .read_file(PROJECT_MANIFEST_PATH)
            .map_err(ProjectError::Storage)?;
        let manifest_text = String::from_utf8(manifest_bytes)
            .map_err(|error| ProjectError::Manifest(error.to_string()))?;
        let manifest = ProjectManifest::parse(&manifest_text).map_err(ProjectError::Manifest)?;
        let lockfile = if storage.exists(PACKAGE_LOCK_PATH) {
            let lock_bytes = storage
                .read_file(PACKAGE_LOCK_PATH)
                .map_err(ProjectError::Storage)?;
            let lock_text = String::from_utf8(lock_bytes)
                .map_err(|error| ProjectError::Lockfile(error.to_string()))?;
            Some(PackageLockFile::parse(&lock_text).map_err(ProjectError::Lockfile)?)
        } else {
            None
        };
        Ok(Self {
            manifest,
            storage,
            lockfile,
        })
    }
    pub fn engine_config(&self) -> EngineConfig {
        let mut config = EngineConfig::new();
        for package in &self.manifest.required_packages {
            config = config.require_package(package.clone());
        }
        if let Some(lock) = &self.lockfile {
            config = config.with_expected_lock(lock.to_lock());
        }
        config
    }
}
