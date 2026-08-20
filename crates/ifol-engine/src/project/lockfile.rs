//! Package lock file for deterministic reproducible project builds.

use crate::package::{PackageId, PackageLock, ResolvedPackage, Version};
use std::fmt;

/// Serialized lock file format representing the exact resolved package graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageLockFile {
    pub format_version: u32,
    pub packages: Vec<ResolvedPackage>,
}

impl PackageLockFile {
    /// Creates a lock file from a resolved `PackageLock`.
    pub fn from_lock(lock: &PackageLock) -> Self {
        Self {
            format_version: 1,
            packages: lock.packages.clone(),
        }
    }

    /// Serializes the lock file into deterministic text.
    pub fn serialize(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("format_version = {}\n", self.format_version));
        for pkg in &self.packages {
            out.push_str("\n[[package]]\n");
            out.push_str(&format!("id = \"{}\"\n", pkg.id));
            out.push_str(&format!("version = \"{}\"\n", pkg.version));
            if !pkg.dependencies.is_empty() {
                out.push_str("dependencies = [");
                let dep_strs: Vec<String> = pkg
                    .dependencies
                    .iter()
                    .map(|d| format!("\"{}\"", d))
                    .collect();
                out.push_str(&dep_strs.join(", "));
                out.push_str("]\n");
            }
        }
        out
    }

    /// Parses a lock file from text.
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut format_version = None;
        let mut packages = Vec::new();

        let mut current_id = None;
        let mut current_version = None;
        let mut current_deps = Vec::new();

        let flush_pkg = |packages: &mut Vec<ResolvedPackage>,
                         current_id: &mut Option<PackageId>,
                         current_version: &mut Option<Version>,
                         current_deps: &mut Vec<PackageId>|
         -> Result<(), String> {
            if let (Some(id), Some(ver)) = (current_id.take(), current_version.take()) {
                packages.push(ResolvedPackage {
                    id,
                    version: ver,
                    dependencies: std::mem::take(current_deps),
                });
            }
            Ok(())
        };

        for line in input.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed == "[[package]]" {
                flush_pkg(
                    &mut packages,
                    &mut current_id,
                    &mut current_version,
                    &mut current_deps,
                )?;
                continue;
            }

            if let Some((k, v)) = trimmed.split_once('=') {
                let key = k.trim();
                let val = v.trim();

                match key {
                    "format_version" => {
                        let ver: u32 = val
                            .parse()
                            .map_err(|_| "invalid format_version in lockfile".to_string())?;
                        format_version = Some(ver);
                    }
                    "id" => {
                        let id_str = val.trim_matches('"');
                        let pkg_id = PackageId::new(id_str)
                            .ok_or_else(|| format!("invalid package id '{id_str}' in lockfile"))?;
                        current_id = Some(pkg_id);
                    }
                    "version" => {
                        let ver_str = val.trim_matches('"');
                        let ver = Version::parse(ver_str).ok_or_else(|| {
                            format!("invalid package version '{ver_str}' in lockfile")
                        })?;
                        current_version = Some(ver);
                    }
                    "dependencies" => {
                        let bracketed = val.trim_matches(|c| c == '[' || c == ']');
                        for part in bracketed.split(',') {
                            let dep_id_str = part.trim().trim_matches('"');
                            if !dep_id_str.is_empty() {
                                let dep_id = PackageId::new(dep_id_str).ok_or_else(|| {
                                    format!("invalid dependency id '{dep_id_str}' in lockfile")
                                })?;
                                current_deps.push(dep_id);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        flush_pkg(
            &mut packages,
            &mut current_id,
            &mut current_version,
            &mut current_deps,
        )?;

        let format_ver = format_version.unwrap_or(1);
        Ok(Self {
            format_version: format_ver,
            packages,
        })
    }
}

impl fmt::Display for PackageLockFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.serialize())
    }
}
