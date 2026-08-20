//! Project storage abstraction and sandbox path security.
//!
//! Enforces containment inside the project bundle. All relative paths are sanitized
//! to prevent directory traversal (`..`), absolute root escapes, or invalid characters.

use std::collections::BTreeMap;
use std::fmt;
use thiserror::Error;

/// Storage errors.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum StorageError {
    #[error("path traversal or invalid path detected: '{0}'")]
    InvalidPath(String),

    #[error("file not found: '{0}'")]
    NotFound(String),

    #[error("I/O error: {0}")]
    Io(String),
}

/// Helper for enforcing project bundle path containment.
pub struct PathSecurity;

impl PathSecurity {
    /// Sanitizes and validates a relative path.
    ///
    /// Rejects:
    /// - Leading slashes `/` or `\`
    /// - `..` parent directory segments
    /// - Windows drive letters (e.g. `C:`)
    /// - Null bytes or invalid characters
    pub fn sanitize(path: &str) -> Result<String, StorageError> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err(StorageError::InvalidPath("empty path".into()));
        }

        // Check drive letters
        if trimmed.len() >= 2 && trimmed.chars().nth(1) == Some(':') {
            return Err(StorageError::InvalidPath(format!(
                "absolute drive letter path rejected: '{path}'"
            )));
        }

        // Normalize slashes
        let normalized = trimmed.replace('\\', "/");
        if normalized.starts_with('/') {
            return Err(StorageError::InvalidPath(format!(
                "absolute path rejected: '{path}'"
            )));
        }

        let segments: Vec<&str> = normalized.split('/').collect();
        let mut clean_segments = Vec::new();

        for seg in segments {
            if seg.is_empty() || seg == "." {
                continue;
            }
            if seg == ".." {
                return Err(StorageError::InvalidPath(format!(
                    "directory traversal '..' rejected: '{path}'"
                )));
            }
            if seg.contains('\0') {
                return Err(StorageError::InvalidPath(format!(
                    "null byte rejected: '{path}'"
                )));
            }
            clean_segments.push(seg);
        }

        if clean_segments.is_empty() {
            return Err(StorageError::InvalidPath("empty relative path".into()));
        }

        Ok(clean_segments.join("/"))
    }
}

/// Abstract storage backend for reading and writing files within a project container.
pub trait ProjectStorage: Send + Sync {
    /// Reads the raw bytes of a file.
    fn read_file(&self, path: &str) -> Result<Vec<u8>, StorageError>;

    /// Writes raw bytes to a file.
    fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), StorageError>;

    /// Lists all relative paths of files in the project.
    fn list_files(&self) -> Result<Vec<String>, StorageError>;

    /// Checks whether a file exists.
    fn exists(&self, path: &str) -> bool;
}

/// In-memory implementation of `ProjectStorage` for testing, sandboxes, and WASM.
#[derive(Debug, Default, Clone)]
pub struct MemoryStorage {
    files: BTreeMap<String, Vec<u8>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
        }
    }
}

impl ProjectStorage for MemoryStorage {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let clean = PathSecurity::sanitize(path)?;
        self.files
            .get(&clean)
            .cloned()
            .ok_or(StorageError::NotFound(clean))
    }

    fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), StorageError> {
        let clean = PathSecurity::sanitize(path)?;
        self.files.insert(clean, data.to_vec());
        Ok(())
    }

    fn list_files(&self) -> Result<Vec<String>, StorageError> {
        Ok(self.files.keys().cloned().collect())
    }

    fn exists(&self, path: &str) -> bool {
        if let Ok(clean) = PathSecurity::sanitize(path) {
            self.files.contains_key(&clean)
        } else {
            false
        }
    }
}

impl fmt::Display for MemoryStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MemoryStorage({} files)", self.files.len())
    }
}
