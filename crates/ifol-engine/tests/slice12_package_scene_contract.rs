//! Slice 12 — package-owned scene schema registration and runtime loading.

use ifol_ecs::entity::EntityId;
use ifol_ecs::world::World;
use ifol_engine::{
    CodecError, CommandRegistry, ComponentCodec, ComponentRecord, EngineBuilder, EntityKey,
    MigrationRegistry, Namespace, PackageId, PackageManifest, PackageRegistration,
    ProjectContainer, RegistrationContext, RegistrationTransaction, SceneDocument, SchemaId,
    SchemaRegistry, Version, VersionReq,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Value(u32);

struct ValueCodec;

struct CountingProvider(Arc<AtomicU32>);

struct LifecycleProvider {
    teardown_calls: Arc<AtomicU32>,
    fail_teardown: bool,
}

impl ifol_engine::ResourceProvider for LifecycleProvider {
    fn id(&self) -> ifol_engine::ResourceId {
        ifol_engine::ResourceId::new("test.lifecycle-provider")
    }

    fn init(&mut self, _ecs: &mut ifol_ecs::EcsRuntime) -> Result<(), ifol_engine::ProviderError> {
        Ok(())
    }

    fn teardown(
        &mut self,
        _ecs: &mut ifol_ecs::EcsRuntime,
    ) -> Result<(), ifol_engine::ProviderError> {
        self.teardown_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_teardown {
            Err(ifol_engine::ProviderError::TeardownFailed {
                provider: self.id().to_string(),
                reason: "test failure".into(),
            })
        } else {
            Ok(())
        }
    }
}

impl ifol_engine::ResourceProvider for CountingProvider {
    fn id(&self) -> ifol_engine::ResourceId {
        ifol_engine::ResourceId::new("test.counting-provider")
    }

    fn init(&mut self, _ecs: &mut ifol_ecs::EcsRuntime) -> Result<(), ifol_engine::ProviderError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn teardown(
        &mut self,
        _ecs: &mut ifol_ecs::EcsRuntime,
    ) -> Result<(), ifol_engine::ProviderError> {
        Ok(())
    }
}

impl ComponentCodec for ValueCodec {
    fn current_version(&self) -> u32 {
        2
    }

    fn encode(&self, world: &World, entity: EntityId) -> Result<Option<Vec<u8>>, CodecError> {
        Ok(world
            .get::<Value>(entity)
            .map(|value| value.0.to_le_bytes().to_vec()))
    }

    fn decode_and_insert(
        &self,
        world: &mut World,
        entity: EntityId,
        data: &[u8],
    ) -> Result<(), CodecError> {
        let bytes: [u8; 4] = data
            .get(..4)
            .ok_or_else(|| CodecError::DecodeFailed {
                schema: "test.value".into(),
                reason: "expected at least four bytes".into(),
            })?
            .try_into()
            .expect("slice length is checked");
        world
            .insert(entity, Value(u32::from_le_bytes(bytes)))
            .map_err(|error| CodecError::DecodeFailed {
                schema: "test.value".into(),
                reason: error.to_string(),
            })
            .map(|_| ())
    }
}

fn manifest(id: &str) -> PackageManifest {
    PackageManifest::new(PackageId::new(id).unwrap(), Version::new(1, 0, 0))
}

fn value_package() -> PackageRegistration<impl FnOnce(&mut RegistrationContext) + Send> {
    PackageRegistration::new(
        manifest("pkg-scene-value"),
        |context: &mut RegistrationContext| {
            context.register_component::<Value>();
            context.register_schema(SchemaId::new("test.value"), Box::new(ValueCodec));
            context.register_migration(
                SchemaId::new("test.value"),
                1,
                2,
                Box::new(|payload: &[u8]| {
                    let mut migrated = payload.to_vec();
                    migrated.extend_from_slice(&0u32.to_le_bytes());
                    Ok(migrated)
                }),
            );
        },
    )
}

#[test]
fn package_schema_and_migration_are_available_through_runtime_contract() {
    let mut engine = EngineBuilder::new()
        .register_package(value_package())
        .build()
        .unwrap();

    assert!(
        engine
            .schema_registry()
            .has_schema(&SchemaId::new("test.value"))
    );
    assert_eq!(engine.schema_registry().len(), 1);
    assert_eq!(engine.revision(), 0);

    let mut scene = SceneDocument::new();
    scene.add_component(
        EntityKey(7),
        ComponentRecord {
            schema: "test.value".into(),
            version: 1,
            payload: 42u32.to_le_bytes().to_vec(),
        },
    );

    let result = engine.load_scene(&scene).unwrap();
    assert_eq!(result.key_to_entity.len(), 1);
    assert!(result.preserved_opaque.is_empty());
    assert_eq!(engine.revision(), 1);
}

#[test]
fn malformed_runtime_scene_does_not_change_revision_or_world() {
    let mut engine = EngineBuilder::new()
        .register_package(value_package())
        .build()
        .unwrap();

    let malformed = SceneDocument {
        entities: vec![EntityKey(1), EntityKey(1)],
        ..SceneDocument::new()
    };

    assert!(engine.load_scene(&malformed).is_err());
    assert_eq!(engine.revision(), 0);
}

#[test]
fn duplicate_schema_across_packages_aborts_build() {
    let duplicate = PackageRegistration::new(
        manifest("pkg-scene-duplicate"),
        |context: &mut RegistrationContext| {
            context.register_schema(SchemaId::new("test.value"), Box::new(ValueCodec));
        },
    );

    let error = EngineBuilder::new()
        .register_package(value_package())
        .register_package(duplicate)
        .build()
        .unwrap_err();

    assert!(error.to_string().contains("schema registration failed"));
}

#[test]
fn successful_reconfiguration_swaps_schema_registry_atomically() {
    let mut engine = EngineBuilder::new()
        .register_package(value_package())
        .build()
        .unwrap();

    let mut transaction = RegistrationTransaction::new();
    transaction.stage_package(PackageId::new("pkg-scene-value").unwrap(), |context| {
        context.register_component::<Value>();
        context.register_schema(SchemaId::new("test.value"), Box::new(ValueCodec));
    });

    let report = engine
        .reconfigure(ifol_engine::ReconfigurationRequest {
            transaction,
            command_registry: CommandRegistry::new(),
            schemas: SchemaRegistry::new(),
            migrations: MigrationRegistry::new(),
            provider_manager: ifol_engine::ProviderManager::new(),
            namespaces: ifol_engine::NamespaceRegistry::new(),
            package_lock: ifol_engine::PackageLock { packages: vec![] },
            added_packages: vec![],
            removed_packages: vec![],
        })
        .unwrap();

    assert_eq!(report.revision, 1);
    assert_eq!(engine.schema_registry().len(), 1);
}

#[test]
fn package_provider_is_staged_and_initialized_after_registration() {
    let initialized = Arc::new(AtomicU32::new(0));
    let provider_counter = initialized.clone();
    let package = PackageRegistration::new(
        manifest("pkg-provider"),
        move |context: &mut RegistrationContext| {
            context.register_provider(Box::new(CountingProvider(provider_counter)));
        },
    );

    let mut engine = EngineBuilder::new()
        .register_package(package)
        .build()
        .unwrap();
    assert_eq!(initialized.load(Ordering::SeqCst), 1);
    engine.shutdown().unwrap();
}

fn empty_reconfiguration_request() -> ifol_engine::ReconfigurationRequest {
    ifol_engine::ReconfigurationRequest {
        transaction: RegistrationTransaction::new(),
        command_registry: CommandRegistry::new(),
        schemas: SchemaRegistry::new(),
        migrations: MigrationRegistry::new(),
        provider_manager: ifol_engine::ProviderManager::new(),
        namespaces: ifol_engine::NamespaceRegistry::new(),
        package_lock: ifol_engine::PackageLock { packages: vec![] },
        added_packages: vec![],
        removed_packages: vec![],
    }
}

#[test]
fn reconfiguration_tears_down_old_providers_before_swap() {
    let teardown_calls = Arc::new(AtomicU32::new(0));
    let calls = teardown_calls.clone();
    let package = PackageRegistration::new(
        manifest("pkg-lifecycle-provider"),
        move |context: &mut RegistrationContext| {
            context.register_provider(Box::new(LifecycleProvider {
                teardown_calls: calls,
                fail_teardown: false,
            }));
        },
    );

    let mut engine = EngineBuilder::new()
        .register_package(package)
        .build()
        .unwrap();
    engine.reconfigure(empty_reconfiguration_request()).unwrap();

    assert_eq!(teardown_calls.load(Ordering::SeqCst), 1);
    assert_eq!(engine.state(), ifol_engine::EngineState::Ready);
}

#[test]
fn provider_teardown_failure_faults_runtime_before_swap() {
    let teardown_calls = Arc::new(AtomicU32::new(0));
    let calls = teardown_calls.clone();
    let package = PackageRegistration::new(
        manifest("pkg-failing-provider"),
        move |context: &mut RegistrationContext| {
            context.register_provider(Box::new(LifecycleProvider {
                teardown_calls: calls,
                fail_teardown: true,
            }));
        },
    );

    let mut engine = EngineBuilder::new()
        .register_package(package)
        .build()
        .unwrap();
    let error = engine
        .reconfigure(empty_reconfiguration_request())
        .unwrap_err();

    assert!(matches!(error, ifol_engine::EngineError::Provider(_)));
    assert_eq!(teardown_calls.load(Ordering::SeqCst), 1);
    assert_eq!(engine.state(), ifol_engine::EngineState::Faulted);
}

#[test]
fn project_package_namespace_claim_is_committed_into_runtime_and_project() {
    let package_id = PackageId::new("pkg-namespace").unwrap();
    let namespace = Namespace::new("pkg.scene").unwrap();
    let package_namespace = namespace.clone();
    let package = PackageRegistration::new(
        manifest("pkg-namespace"),
        move |context: &mut RegistrationContext| {
            context.claim_project_namespace(package_namespace);
        },
    );

    let project = ProjectContainer::new_memory("namespace-project", "scenes/main");
    let project = ProjectContainer {
        manifest: project
            .manifest
            .with_package(package_id, VersionReq::caret(Version::new(1, 0, 0))),
        ..project
    };

    let engine = EngineBuilder::new()
        .register_package(package)
        .with_project(project)
        .build()
        .unwrap();

    assert_eq!(engine.namespace_registry().len(), 1);
    assert_eq!(
        engine
            .namespace_registry()
            .get_owner(&namespace)
            .unwrap()
            .as_str(),
        "pkg-namespace"
    );
    assert_eq!(engine.project().unwrap().namespaces.len(), 1);
}

#[test]
fn colliding_package_namespaces_abort_build_before_runtime_publish() {
    let first = PackageRegistration::new(
        manifest("pkg-namespace-a"),
        |context: &mut RegistrationContext| {
            context.claim_project_namespace(Namespace::new("shared").unwrap());
        },
    );
    let second = PackageRegistration::new(
        manifest("pkg-namespace-b"),
        |context: &mut RegistrationContext| {
            context.claim_project_namespace(Namespace::new("shared.child").unwrap());
        },
    );

    let error = EngineBuilder::new()
        .register_package(first)
        .register_package(second)
        .build()
        .unwrap_err();
    assert!(error.to_string().contains("namespace claim failed"));
}
