use ifol_engine::{
    EngineBuilder, EngineConfig, EngineError, PackageDependency, PackageId, PackageManifest,
    PackageRegistration, PackageResolver, RegistrationContext, Version, VersionReq,
};

fn register_noop(_: &mut RegistrationContext) {}

fn package(id: &str) -> PackageRegistration<fn(&mut RegistrationContext)> {
    PackageRegistration::new(
        PackageManifest::new(PackageId::new(id).unwrap(), Version::new(1, 0, 0)),
        register_noop,
    )
}

#[test]
fn config_activates_only_required_transitive_closure() {
    let root = PackageManifest::new(PackageId::new("root").unwrap(), Version::new(1, 0, 0))
        .with_dependency(PackageDependency {
            package_id: PackageId::new("dep").unwrap(),
            version_req: VersionReq::caret(Version::new(1, 0, 0)),
        });
    let config = EngineConfig::new().require_package(PackageDependency {
        package_id: PackageId::new("root").unwrap(),
        version_req: VersionReq::caret(Version::new(1, 0, 0)),
    });
    let engine = EngineBuilder::new()
        .with_config(config)
        .register_package(PackageRegistration::new(root, register_noop))
        .register_package(package("dep"))
        .register_package(package("unused"))
        .build()
        .unwrap();
    assert_eq!(engine.package_lock().len(), 2);
    assert!(
        engine
            .package_lock()
            .find(&PackageId::new("unused").unwrap())
            .is_none()
    );
}

#[test]
fn config_rejects_lock_drift_before_runtime_publish() {
    let stale = PackageResolver::new().resolve().unwrap();
    let error = EngineBuilder::new()
        .with_config(EngineConfig::new().with_expected_lock(stale))
        .register_package(package("actual"))
        .build()
        .unwrap_err();
    assert!(matches!(error, EngineError::BuildFailed { .. }));
}
