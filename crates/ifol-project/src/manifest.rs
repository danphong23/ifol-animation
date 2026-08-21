use ifol_engine::{PackageDependency, PackageId, VersionReq};
use std::fmt;

pub const CURRENT_FORMAT_VERSION: u32 = 1;

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

impl fmt::Display for ProjectManifest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.serialize())
    }
}
