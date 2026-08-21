#![doc = include_str!("../docs/README.md")]

mod container;
mod lockfile;
mod manifest;
mod storage;

pub use container::{ProjectContainer, ProjectError};
pub use lockfile::PackageLockFile;
pub use manifest::{CURRENT_FORMAT_VERSION, ProjectManifest};
pub use storage::{
    MemoryStorage, PACKAGE_LOCK_PATH, PROJECT_MANIFEST_PATH, ProjectStorage, StorageError,
    sanitize_path,
};

#[cfg(test)]
mod tests;
