use ifol_ecs::{AccessDescriptor, PhaseId, SystemContext};
use ifol_engine::{
    CommandRegistry, EngineBuilder, EngineConfig, EngineError, EngineState, MigrationRegistry,
    NamespaceRegistry, PackageDependency, PackageId, PackageLock, PackageManifest,
    PackageRegistration, ReconfigurationRequest, RegistrationTransaction, SceneDocument, SceneId,
    SchemaRegistry, StepInput, Version, VersionReq,
};
use ifol_project::{PackageLockFile, ProjectContainer};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

fn no_op(_: &mut ifol_engine::RegistrationContext) {}

fn package(
    id: &str,
    counter: Arc<AtomicU32>,
) -> PackageRegistration<impl FnOnce(&mut ifol_engine::RegistrationContext) + Send> {
    let id = PackageId::new(id).unwrap();
    let phase = PhaseId::new("update");
    PackageRegistration::new(
        PackageManifest::new(id, Version::new(1, 0, 0)),
        move |ctx: &mut ifol_engine::RegistrationContext| {
            ctx.register_phase(phase.clone());
            ctx.register_system(
                "tick",
                phase,
                move |_: &mut SystemContext<'_>| {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                },
                AccessDescriptor::new(),
                vec![],
            );
        },
    )
}

fn empty_reconfiguration() -> ReconfigurationRequest {
    ReconfigurationRequest {
        transaction: RegistrationTransaction::new(),
        command_registry: CommandRegistry::new(),
        schemas: SchemaRegistry::new(),
        migrations: MigrationRegistry::new(),
        provider_manager: ifol_engine::ProviderManager::new(),
        namespaces: NamespaceRegistry::new(),
        package_lock: PackageLock { packages: vec![] },
        added_packages: vec![],
        removed_packages: vec![],
    }
}

#[test]
fn tc01_project_bootstrap_translates_storage_to_headless_runtime() {
    let id = PackageId::new("pkg-project").unwrap();
    let mut project = ProjectContainer::new_memory("demo", "scenes/main");
    project.manifest = project
        .manifest
        .clone()
        .with_package(id.clone(), VersionReq::caret(Version::new(1, 0, 0)));
    project.save().unwrap();
    let project = ProjectContainer::load(project.storage).unwrap();
    let engine = EngineBuilder::new()
        .with_config(project.engine_config())
        .register_package(PackageRegistration::new(
            PackageManifest::new(id, Version::new(1, 0, 0)),
            no_op,
        ))
        .build()
        .unwrap();
    assert_eq!(engine.state(), EngineState::Ready);
}

#[test]
fn tc02_project_lock_must_match_resolved_closure() {
    let id = PackageId::new("pkg-lock").unwrap();
    let mut project = ProjectContainer::new_memory("demo", "main");
    project.manifest = project
        .manifest
        .clone()
        .with_package(id.clone(), VersionReq::caret(Version::new(1, 0, 0)));
    project.lockfile = Some(PackageLockFile {
        format_version: 1,
        packages: vec![],
    });
    let error = EngineBuilder::new()
        .with_config(project.engine_config())
        .register_package(PackageRegistration::new(
            PackageManifest::new(id, Version::new(1, 0, 0)),
            no_op,
        ))
        .build()
        .unwrap_err();
    assert!(matches!(error, EngineError::BuildFailed { .. }));
}

#[test]
fn tc03_registered_package_executes_through_project_config() {
    let counter = Arc::new(AtomicU32::new(0));
    let id = PackageId::new("pkg-step").unwrap();
    let config = EngineConfig::new().require_package(PackageDependency {
        package_id: id,
        version_req: VersionReq::caret(Version::new(1, 0, 0)),
    });
    let mut engine = EngineBuilder::new()
        .with_config(config)
        .register_package(package("pkg-step", counter.clone()))
        .build()
        .unwrap();
    engine.step(StepInput::default()).unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn tc04_scene_lifecycle_is_runtime_owned_and_does_not_save_project_files() {
    let mut engine = EngineBuilder::new().build().unwrap();
    let scene = SceneDocument::new();
    let loaded = engine
        .load_scene_as(SceneId::new("main").unwrap(), &scene)
        .unwrap();
    assert!(loaded.key_to_entity.is_empty());
    assert_eq!(engine.active_scene(), Some(&SceneId::new("main").unwrap()));
    assert!(engine.clear_scene().unwrap());
    assert_eq!(engine.active_scene(), None);
}

#[test]
fn tc05_reconfigure_replaces_composition_and_resets_ecs_runtime() {
    let counter = Arc::new(AtomicU32::new(0));
    let mut engine = EngineBuilder::new()
        .register_package(package("pkg-live", counter.clone()))
        .build()
        .unwrap();
    engine.step(StepInput::default()).unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    let report = engine.reconfigure(empty_reconfiguration()).unwrap();
    assert_eq!(report.revision, 2);
    assert_eq!(engine.state(), EngineState::Ready);
    assert!(engine.step(StepInput::default()).is_ok());
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn tc06_failed_reconfigure_preserves_ready_state_and_shutdown_is_terminal() {
    let mut engine = EngineBuilder::new().build().unwrap();
    let mut tx = RegistrationTransaction::new();
    let p1 = PhaseId::new("p1");
    let p2 = PhaseId::new("p2");
    tx.stage_package(PackageId::new("pkg-cycle").unwrap(), |ctx| {
        ctx.register_phase(p1.clone());
        ctx.register_phase(p2.clone());
        ctx.add_phase_edge(p1.clone(), p2.clone());
        ctx.add_phase_edge(p2, p1.clone());
    });
    let error = engine
        .reconfigure(ReconfigurationRequest {
            transaction: tx,
            command_registry: CommandRegistry::new(),
            schemas: SchemaRegistry::new(),
            migrations: MigrationRegistry::new(),
            provider_manager: ifol_engine::ProviderManager::new(),
            namespaces: NamespaceRegistry::new(),
            package_lock: PackageLock { packages: vec![] },
            added_packages: vec![],
            removed_packages: vec![],
        })
        .unwrap_err();
    assert!(error.to_string().contains("cycle") || error.to_string().contains("phase"));
    assert_eq!(engine.state(), EngineState::Ready);
    engine.shutdown().unwrap();
    assert_eq!(engine.state(), EngineState::ShuttingDown);
}
