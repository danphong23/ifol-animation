use std::collections::BTreeMap;
use thiserror::Error;

/// Canonical project manifest path.
pub const PROJECT_MANIFEST_PATH: &str = "project.toml";
/// Canonical package lock path.
pub const PACKAGE_LOCK_PATH: &str = "package.lock";

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum StorageError {
    #[error("invalid project path: {0}")]
    InvalidPath(String),
    #[error("file not found: {0}")]
    NotFound(String),
}

/// Validates a project-relative path without touching the host filesystem.
pub fn sanitize_path(path: &str) -> Result<String, StorageError> {
    let path = path.trim().replace('\\', "/");
    if path.is_empty() || path.starts_with('/') || (path.len() > 1 && path.as_bytes()[1] == b':') {
        return Err(StorageError::InvalidPath(path));
    }
    let mut clean = Vec::new();
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." || segment.contains('\0') {
            return Err(StorageError::InvalidPath(path));
        }
        clean.push(segment);
    }
    if clean.is_empty() {
        return Err(StorageError::InvalidPath(path));
    }
    Ok(clean.join("/"))
}

pub trait ProjectStorage: Send + Sync {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, StorageError>;
    fn write_files(&mut self, files: &[(&str, Vec<u8>)]) -> Result<(), StorageError>;
    fn exists(&self, path: &str) -> bool;
}

#[derive(Debug, Default, Clone)]
pub struct MemoryStorage {
    files: BTreeMap<String, Vec<u8>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ProjectStorage for MemoryStorage {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let path = sanitize_path(path)?;
        self.files
            .get(&path)
            .cloned()
            .ok_or(StorageError::NotFound(path))
    }
    fn write_files(&mut self, files: &[(&str, Vec<u8>)]) -> Result<(), StorageError> {
        let mut next = self.files.clone();
        for (path, data) in files {
            next.insert(sanitize_path(path)?, data.clone());
        }
        self.files = next;
        Ok(())
    }
    fn exists(&self, path: &str) -> bool {
        sanitize_path(path)
            .ok()
            .is_some_and(|path| self.files.contains_key(&path))
    }
}
