//! Slice 11 — project selection and runtime ownership.

mod support;

use ifol_engine::{
    EngineBuilder, EngineError, PackageId, PackageLockFile, PackageManifest, ProjectContainer,
    Version, VersionReq,
};
use std::sync::{Arc, Mutex};
use support::{inline_package, inline_package_with_dependency};

fn required(id: &str) -> (PackageId, VersionReq) {
    (
        PackageId::new(id).unwrap(),
        VersionReq::caret(Version::new(1, 0, 0)),
    )
}

#[test]
fn project_activates_only_required_transitive_package_closure() {
    let activation = Arc::new(Mutex::new(Vec::<String>::new()));
    let alpha_activation = Arc::clone(&activation);
    let beta_activation = Arc::clone(&activation);
    let unused_activation = Arc::clone(&activation);

    let mut project = ProjectContainer::new_memory("demo", "scenes/main.ifol");
    let (beta_id, beta_req) = required("pkg-beta");
    project.manifest = project.manifest.with_package(beta_id, beta_req);

    let alpha_id = PackageId::new("pkg-alpha").unwrap();
    let beta_id = PackageId::new("pkg-beta").unwrap();
    let unused_id = PackageId::new("pkg-unused").unwrap();

    let alpha = inline_package(alpha_id.clone(), move |_| {
        alpha_activation.lock().unwrap().push("alpha".into());
    });
    let beta = inline_package_with_dependency(beta_id, alpha_id, move |_| {
        beta_activation.lock().unwrap().push("beta".into());
    });
    let unused = inline_package(unused_id, move |_| {
        unused_activation.lock().unwrap().push("unused".into());
    });

    let engine = EngineBuilder::new()
        .with_project(project)
        .register_package(unused)
        .register_package(beta)
        .register_package(alpha)
        .build()
        .unwrap();

    assert_eq!(activation.lock().unwrap().as_slice(), ["alpha", "beta"]);
    assert_eq!(engine.package_lock().len(), 2);
    assert_eq!(engine.project().unwrap().manifest.name, "demo");
}

#[test]
fn missing_project_root_fails_before_any_package_registration() {
    let activation = Arc::new(Mutex::new(0_u32));
    let observed = Arc::clone(&activation);
    let mut project = ProjectContainer::new_memory("demo", "scenes/main.ifol");
    let (missing_id, missing_req) = required("pkg-missing");
    project.manifest = project.manifest.with_package(missing_id, missing_req);

    let package = inline_package(PackageId::new("pkg-available").unwrap(), move |_| {
        *observed.lock().unwrap() += 1;
    });

    let error = EngineBuilder::new()
        .with_project(project)
        .register_package(package)
        .build()
        .unwrap_err();

    assert!(matches!(error, EngineError::Resolution(_)));
    assert_eq!(*activation.lock().unwrap(), 0);
}

#[test]
fn stale_project_lockfile_is_rejected_before_registration() {
    let mut project = ProjectContainer::new_memory("demo", "scenes/main.ifol");
    let (required_id, required_req) = required("pkg-alpha");
    project.manifest = project.manifest.with_package(required_id, required_req);
    project.lockfile = Some(PackageLockFile {
        format_version: 1,
        packages: Vec::new(),
    });

    let package = inline_package(PackageId::new("pkg-alpha").unwrap(), |_| {});
    let error = EngineBuilder::new()
        .with_project(project)
        .register_package(package)
        .build()
        .unwrap_err();

    assert!(matches!(
        error,
        EngineError::Project(ifol_engine::ProjectError::LockMismatch { .. })
    ));
}

#[test]
fn project_lockfile_matching_resolved_closure_is_accepted() {
    let mut project = ProjectContainer::new_memory("demo", "scenes/main.ifol");
    let (required_id, required_req) = required("pkg-alpha");
    project.manifest = project.manifest.with_package(required_id, required_req);

    let package = inline_package(PackageId::new("pkg-alpha").unwrap(), |_| {});
    let mut resolver = ifol_engine::PackageResolver::new();
    resolver.add(PackageManifest::new(
        PackageId::new("pkg-alpha").unwrap(),
        Version::new(1, 0, 0),
    ));
    project.lockfile = Some(PackageLockFile::from_lock(&resolver.resolve().unwrap()));

    let engine = EngineBuilder::new()
        .with_project(project)
        .register_package(package)
        .build()
        .unwrap();
    assert_eq!(engine.package_lock().len(), 1);
}
