//! Slice 10 — package contract and builder resolution integration.
//!
//! These tests prove that package manifests are resolved before registration,
//! that activation order follows the deterministic lock, and that malformed
//! candidate sets fail before an engine becomes observable.

use ifol_engine::{
    EngineBuilder, EngineError, EnginePackage, PackageDependency, PackageError, PackageId,
    PackageManifest, PackageRegistration, RegistrationContext, Version, VersionReq,
};
use std::sync::{Arc, Mutex};

fn package(
    id: &str,
    register: impl for<'a> FnOnce(&'a mut RegistrationContext) + Send + 'static,
) -> PackageRegistration<impl for<'a> FnOnce(&'a mut RegistrationContext) + Send + 'static> {
    PackageRegistration::new(
        PackageManifest::new(PackageId::new(id).unwrap(), Version::new(1, 0, 0)),
        register,
    )
}

#[test]
fn builder_resolves_dependencies_before_registration_and_exposes_lock() {
    let activation = Arc::new(Mutex::new(Vec::<String>::new()));
    let alpha_activation = Arc::clone(&activation);
    let beta_activation = Arc::clone(&activation);

    let alpha = package("pkg-alpha", move |_context: &mut RegistrationContext| {
        alpha_activation.lock().unwrap().push("alpha".into());
    });
    let beta_manifest =
        PackageManifest::new(PackageId::new("pkg-beta").unwrap(), Version::new(1, 0, 0))
            .with_dependency(PackageDependency {
                package_id: PackageId::new("pkg-alpha").unwrap(),
                version_req: VersionReq::caret(Version::new(1, 0, 0)),
            });
    let beta =
        PackageRegistration::new(beta_manifest, move |_context: &mut RegistrationContext| {
            beta_activation.lock().unwrap().push("beta".into());
        });

    let engine = EngineBuilder::new()
        // Reverse discovery order must not change dependency activation order.
        .register_package(beta)
        .register_package(alpha)
        .build()
        .unwrap();

    assert_eq!(activation.lock().unwrap().as_slice(), ["alpha", "beta"]);
    assert_eq!(
        engine
            .package_lock()
            .packages
            .iter()
            .map(|package| package.id.as_str())
            .collect::<Vec<_>>(),
        ["pkg-alpha", "pkg-beta"]
    );
}

#[test]
fn builder_rejects_duplicate_package_ids_before_registration() {
    let first = package("pkg-duplicate", |_context: &mut RegistrationContext| {});
    let second = package("pkg-duplicate", |_context: &mut RegistrationContext| {});

    let error = EngineBuilder::new()
        .register_package(first)
        .register_package(second)
        .build()
        .unwrap_err();

    assert!(matches!(error, EngineError::Resolution(_)));
}

#[test]
fn builder_rejects_missing_dependency_before_registration() {
    let manifest = PackageManifest::new(
        PackageId::new("pkg-dependent").unwrap(),
        Version::new(1, 0, 0),
    )
    .with_dependency(PackageDependency {
        package_id: PackageId::new("pkg-missing").unwrap(),
        version_req: VersionReq::caret(Version::new(1, 0, 0)),
    });
    let package = PackageRegistration::new(manifest, |_context: &mut RegistrationContext| {});

    let error = EngineBuilder::new()
        .register_package(package)
        .build()
        .unwrap_err();

    assert!(matches!(error, EngineError::Resolution(_)));
}

struct FailingPackage {
    manifest: PackageManifest,
}

impl EnginePackage for FailingPackage {
    fn manifest(&self) -> &PackageManifest {
        &self.manifest
    }

    fn register(&self, _context: &mut RegistrationContext) -> Result<(), PackageError> {
        Err(PackageError::Registration("fixture failure".into()))
    }
}

#[test]
fn package_preparation_failure_is_typed_and_no_runtime_is_returned() {
    let package = FailingPackage {
        manifest: PackageManifest::new(PackageId::new("pkg-fail").unwrap(), Version::new(1, 0, 0)),
    };

    let error = EngineBuilder::new()
        .register_package(package)
        .build()
        .unwrap_err();

    assert!(matches!(
        error,
        EngineError::PackagePreparation { package, reason }
            if package.as_str() == "pkg-fail" && reason.contains("fixture failure")
    ));
}
