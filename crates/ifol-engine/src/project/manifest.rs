use crate::package::{PackageDependency, PackageId, VersionReq};
use std::fmt;

/// Supported project bundle format version.
pub const CURRENT_FORMAT_VERSION: u32 = 1;

/// Manifest header describing a project container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectManifest {
    /// Bundle container format specification version.
    pub format_version: u32,
    /// Human-readable project name.
    pub name: String,
    /// Packages required by this project with version constraints.
    pub required_packages: Vec<PackageDependency>,
    /// Path or key of the default entry scene.
    pub entry_scene: String,
}

impl ProjectManifest {
    /// Creates a new minimal project manifest.
    pub fn new(name: impl Into<String>, entry_scene: impl Into<String>) -> Self {
        Self {
            format_version: CURRENT_FORMAT_VERSION,
            name: name.into(),
            required_packages: Vec::new(),
            entry_scene: entry_scene.into(),
        }
    }

    /// Adds a required package dependency.
    pub fn with_package(mut self, id: PackageId, version_req: VersionReq) -> Self {
        self.required_packages.push(PackageDependency {
            package_id: id,
            version_req,
        });
        self
    }

    /// Validates the manifest format version and required fields.
    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != CURRENT_FORMAT_VERSION {
            return Err(format!(
                "unsupported project format version {}, expected {CURRENT_FORMAT_VERSION}",
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

    /// Serializes manifest to a deterministic textual format.
    pub fn serialize(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("format_version = {}\n", self.format_version));
        out.push_str(&format!("name = \"{}\"\n", quote_string(&self.name)));
        out.push_str(&format!(
            "entry_scene = \"{}\"\n",
            quote_string(&self.entry_scene)
        ));
        out.push_str("[packages]\n");
        for pkg in &self.required_packages {
            out.push_str(&format!("{} = \"{}\"\n", pkg.package_id, pkg.version_req));
        }
        out
    }

    /// Deserializes manifest from a textual format.
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut format_version = None;
        let mut name = None;
        let mut entry_scene = None;
        let mut required_packages = Vec::new();
        let mut in_packages_section = false;

        for line in input.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed == "[packages]" {
                in_packages_section = true;
                continue;
            }

            if let Some((k, v)) = trimmed.split_once('=') {
                let key = k.trim();
                let raw_value = v.trim();

                if in_packages_section {
                    let val = raw_value.trim_matches('"');
                    let pkg_id =
                        PackageId::new(key).ok_or_else(|| format!("invalid package id '{key}'"))?;
                    let req = VersionReq::parse(val).ok_or_else(|| {
                        format!("invalid version requirement '{val}' for '{key}'")
                    })?;
                    required_packages.push(PackageDependency {
                        package_id: pkg_id,
                        version_req: req,
                    });
                } else {
                    match key {
                        "format_version" => {
                            let ver: u32 = raw_value
                                .parse()
                                .map_err(|_| "invalid format_version integer".to_string())?;
                            format_version = Some(ver);
                        }
                        "name" => name = Some(parse_string(raw_value)?),
                        "entry_scene" => entry_scene = Some(parse_string(raw_value)?),
                        _ => {}
                    }
                }
            }
        }

        let manifest = Self {
            format_version: format_version
                .ok_or_else(|| "missing format_version in manifest".to_string())?,
            name: name.ok_or_else(|| "missing name in manifest".to_string())?,
            entry_scene: entry_scene
                .ok_or_else(|| "missing entry_scene in manifest".to_string())?,
            required_packages,
        };

        manifest.validate()?;
        Ok(manifest)
    }
}

fn quote_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn parse_string(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err("expected quoted string".into());
    }
    let inner = &value[1..value.len() - 1];
    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }
        match chars.next() {
            Some('\\') => result.push('\\'),
            Some('"') => result.push('"'),
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('t') => result.push('\t'),
            Some(other) => return Err(format!("unsupported string escape '\\{other}'")),
            None => return Err("unterminated string escape".into()),
        }
    }
    Ok(result)
}

impl fmt::Display for ProjectManifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.serialize())
    }
}
