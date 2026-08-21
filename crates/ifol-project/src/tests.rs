use crate::{PackageLockFile, ProjectContainer, ProjectManifest};
use ifol_engine::{PackageId, PackageManifest, Version, VersionReq};

fn register_noop(_: &mut ifol_engine::RegistrationContext) {}

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

#[test]
fn escaped_manifest_values_round_trip_without_filesystem() {
    let manifest = ProjectManifest::new("demo \"project\"", "scenes/main\nnext").with_package(
        PackageId::new("pkg.demo").unwrap(),
        VersionReq::caret(Version::new(1, 2, 3)),
    );
    assert_eq!(
        ProjectManifest::parse(&manifest.serialize()).unwrap(),
        manifest
    );
}

#[test]
fn loaded_project_config_builds_a_headless_engine() {
    let package_id = PackageId::new("pkg.project").unwrap();
    let mut project = ProjectContainer::new_memory("demo", "scenes/main");
    project.manifest = project
        .manifest
        .clone()
        .with_package(package_id.clone(), VersionReq::caret(Version::new(1, 0, 0)));
    let package = ifol_engine::PackageRegistration::new(
        PackageManifest::new(package_id, Version::new(1, 0, 0)),
        register_noop,
    );
    let engine = ifol_engine::EngineBuilder::new()
        .with_config(project.engine_config())
        .register_package(package)
        .build()
        .unwrap();
    assert_eq!(engine.state(), ifol_engine::EngineState::Ready);
    assert_eq!(engine.package_lock().len(), 1);
}

#[test]
fn lockfile_round_trip_preserves_empty_lock() {
    let lock = PackageLockFile {
        format_version: 1,
        packages: Vec::new(),
    };
    assert_eq!(PackageLockFile::parse(&lock.serialize()).unwrap(), lock);
}
