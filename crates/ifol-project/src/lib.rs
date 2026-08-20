//! Project persistence and host-side project composition.
//!
//! This crate owns project files and storage backends. It translates a loaded
//! project into [`ifol_engine::EngineConfig`]; the engine itself remains
//! unaware of files, paths, manifests, and lock-file syntax.

use ifol_engine::{EngineConfig, PackageDependency, PackageId, PackageLock, Version, VersionReq};
use std::collections::BTreeMap;
use std::fmt;
use thiserror::Error;

/// Canonical project files.
pub const PROJECT_MANIFEST_PATH: &str = "project.toml";
pub const PACKAGE_LOCK_PATH: &str = "package.lock";
pub const CURRENT_FORMAT_VERSION: u32 = 1;

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
            .is_some_and(|p| self.files.contains_key(&p))
    }
}

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ProjectError {
    #[error("manifest error: {0}")]
    Manifest(String),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("lockfile error: {0}")]
    Lockfile(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectManifest {
    pub format_version: u32,
    pub name: String,
    pub required_packages: Vec<PackageDependency>,
    pub entry_scene: String,
}

impl ProjectManifest {
    pub fn new(name: impl Into<String>, entry_scene: impl Into<String>) -> Self {
        Self {
            format_version: CURRENT_FORMAT_VERSION,
            name: name.into(),
            required_packages: Vec::new(),
            entry_scene: entry_scene.into(),
        }
    }
    pub fn with_package(mut self, id: PackageId, version_req: VersionReq) -> Self {
        self.required_packages.push(PackageDependency {
            package_id: id,
            version_req,
        });
        self
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != CURRENT_FORMAT_VERSION {
            return Err(format!(
                "unsupported project format version {}",
                self.format_version
            ));
        }
        if self.name.trim().is_empty() {
            return Err("project name cannot be empty".into());
        }
        if self.entry_scene.trim().is_empty() {
            return Err("entry scene cannot be empty".into());
        }
        Ok(())
    }
    pub fn serialize(&self) -> String {
        let mut out = format!(
            "format_version = {}\nname = \"{}\"\nentry_scene = \"{}\"\n[packages]\n",
            self.format_version,
            quote(&self.name),
            quote(&self.entry_scene)
        );
        for package in &self.required_packages {
            out.push_str(&format!(
                "{} = \"{}\"\n",
                package.package_id, package.version_req
            ));
        }
        out
    }
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut format_version = None;
        let mut name = None;
        let mut entry_scene = None;
        let mut required_packages = Vec::new();
        let mut packages = false;
        for line in input.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line == "[packages]" {
                packages = true;
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            if packages {
                let id = PackageId::new(key.trim())
                    .ok_or_else(|| format!("invalid package id '{}'", key.trim()))?;
                let req = VersionReq::parse(value.trim_matches('"'))
                    .ok_or_else(|| "invalid package version requirement".to_string())?;
                required_packages.push(PackageDependency {
                    package_id: id,
                    version_req: req,
                });
            } else {
                match key.trim() {
                    "format_version" => {
                        format_version = Some(
                            value
                                .parse()
                                .map_err(|_| "invalid format version".to_string())?,
                        )
                    }
                    "name" => name = Some(unquote(value)?),
                    "entry_scene" => entry_scene = Some(unquote(value)?),
                    _ => {}
                }
            }
        }
        let manifest = Self {
            format_version: format_version.ok_or("missing format version")?,
            name: name.ok_or("missing name")?,
            entry_scene: entry_scene.ok_or("missing entry scene")?,
            required_packages,
        };
        manifest.validate()?;
        Ok(manifest)
    }
}

fn quote(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
fn unquote(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err("expected quoted string".into());
    }
    let mut out = String::new();
    let mut chars = value[1..value.len() - 1].chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            _ => return Err("invalid string escape".into()),
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageLockFile {
    pub format_version: u32,
    pub packages: Vec<ifol_engine::ResolvedPackage>,
}

impl PackageLockFile {
    pub fn from_lock(lock: &PackageLock) -> Self {
        Self {
            format_version: 1,
            packages: lock.packages.clone(),
        }
    }
    pub fn to_lock(&self) -> PackageLock {
        PackageLock {
            packages: self.packages.clone(),
        }
    }
    pub fn serialize(&self) -> String {
        let mut out = format!("format_version = {}\n", self.format_version);
        for package in &self.packages {
            out.push_str(&format!(
                "\n[[package]]\nid = \"{}\"\nversion = \"{}\"\n",
                package.id, package.version
            ));
            if !package.dependencies.is_empty() {
                out.push_str(&format!(
                    "dependencies = [{}]\n",
                    package
                        .dependencies
                        .iter()
                        .map(|id| format!("\"{id}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        out
    }
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut format_version = 1;
        let mut packages = Vec::new();
        let mut id = None;
        let mut version = None;
        let mut dependencies = Vec::new();
        let flush = |packages: &mut Vec<ifol_engine::ResolvedPackage>,
                     id: &mut Option<PackageId>,
                     version: &mut Option<Version>,
                     dependencies: &mut Vec<PackageId>| {
            if let (Some(id), Some(version)) = (id.take(), version.take()) {
                packages.push(ifol_engine::ResolvedPackage {
                    id,
                    version,
                    dependencies: std::mem::take(dependencies),
                });
            }
        };
        for line in input.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line == "[[package]]" {
                flush(&mut packages, &mut id, &mut version, &mut dependencies);
                continue;
            }
            let Some((key, raw)) = line.split_once('=') else {
                continue;
            };
            let value = raw.trim();
            match key.trim() {
                "format_version" => {
                    format_version = value
                        .parse()
                        .map_err(|_| "invalid format version".to_string())?
                }
                "id" => {
                    id = Some(PackageId::new(value.trim_matches('"')).ok_or("invalid package id")?)
                }
                "version" => {
                    version = Some(
                        Version::parse(value.trim_matches('"')).ok_or("invalid package version")?,
                    )
                }
                "dependencies" => {
                    for part in value.trim_matches(|c| c == '[' || c == ']').split(',') {
                        let part = part.trim().trim_matches('"');
                        if !part.is_empty() {
                            dependencies.push(PackageId::new(part).ok_or("invalid dependency id")?);
                        }
                    }
                }
                _ => {}
            }
        }
        flush(&mut packages, &mut id, &mut version, &mut dependencies);
        Ok(Self {
            format_version,
            packages,
        })
    }
}

impl fmt::Display for ProjectManifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.serialize())
    }
}
impl fmt::Display for PackageLockFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.serialize())
    }
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
        let manifest = ProjectManifest::parse(
            &String::from_utf8(
                storage
                    .read_file(PROJECT_MANIFEST_PATH)
                    .map_err(ProjectError::Storage)?,
            )
            .map_err(|e| ProjectError::Manifest(e.to_string()))?,
        )
        .map_err(ProjectError::Manifest)?;
        let lockfile = if storage.exists(PACKAGE_LOCK_PATH) {
            Some(
                PackageLockFile::parse(
                    &String::from_utf8(
                        storage
                            .read_file(PACKAGE_LOCK_PATH)
                            .map_err(ProjectError::Storage)?,
                    )
                    .map_err(|e| ProjectError::Lockfile(e.to_string()))?,
                )
                .map_err(ProjectError::Lockfile)?,
            )
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_roundtrip_and_engine_config() {
        let id = PackageId::new("demo").unwrap();
        let mut project = ProjectContainer::new_memory("demo", "main");
        project.manifest = project
            .manifest
            .clone()
            .with_package(id, VersionReq::caret(Version::new(1, 0, 0)));
        project.save().unwrap();
        let loaded = ProjectContainer::load(project.storage).unwrap();
        assert_eq!(loaded.manifest, project.manifest);
        assert_eq!(loaded.engine_config().required_package_count(), 1);
    }
}
