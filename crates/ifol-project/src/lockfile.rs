use ifol_engine::{PackageId, PackageLock, ResolvedPackage, Version};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageLockFile {
    pub format_version: u32,
    pub packages: Vec<ResolvedPackage>,
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
                    for part in value
                        .trim_matches(|character| character == '[' || character == ']')
                        .split(',')
                    {
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

fn flush(
    packages: &mut Vec<ResolvedPackage>,
    id: &mut Option<PackageId>,
    version: &mut Option<Version>,
    dependencies: &mut Vec<PackageId>,
) {
    if let (Some(id), Some(version)) = (id.take(), version.take()) {
        packages.push(ResolvedPackage {
            id,
            version,
            dependencies: std::mem::take(dependencies),
        });
    }
}

impl fmt::Display for PackageLockFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.serialize())
    }
}
