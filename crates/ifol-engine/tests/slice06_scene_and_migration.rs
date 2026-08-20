//! Slice 6 — Scene Document, Schema, Migration & Transactional Loader
//!
//! Acceptance criteria:
//! - Scene document creation and component records
//! - Component codec for encoding/decoding components onto ECS entities
//! - Multi-step schema migration chains (v1 -> v2 -> v3) and migration gap detection
//! - Preserving unknown component schemas as opaque records without data loss
//! - Fail-closed rollback on scene loading failure

use ifol_ecs::entity::EntityId;
use ifol_ecs::world::World;
use ifol_engine::{
    CodecError, ComponentCodec, ComponentRecord, EntityKey, MigrationError, MigrationRegistry,
    SceneDocument, SceneError, SceneLoader, SchemaId, SchemaRegistry,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Transform {
    x: i32,
    y: i32,
    scale: u32,
}

struct TransformCodec;

impl ComponentCodec for TransformCodec {
    fn current_version(&self) -> u32 {
        3 // latest version
    }

    fn encode(&self, world: &World, entity: EntityId) -> Result<Option<Vec<u8>>, CodecError> {
        if let Some(t) = world.get::<Transform>(entity) {
            // v3 format: 4 bytes x, 4 bytes y, 4 bytes scale
            let mut buf = Vec::new();
            buf.extend_from_slice(&t.x.to_le_bytes());
            buf.extend_from_slice(&t.y.to_le_bytes());
            buf.extend_from_slice(&t.scale.to_le_bytes());
            Ok(Some(buf))
        } else {
            Ok(None)
        }
    }

    fn decode_and_insert(
        &self,
        world: &mut World,
        entity: EntityId,
        data: &[u8],
    ) -> Result<(), CodecError> {
        if data.len() < 12 {
            return Err(CodecError::DecodeFailed {
                schema: "core.transform".into(),
                reason: "data too short for v3 transform".into(),
            });
        }
        let x = i32::from_le_bytes(data[0..4].try_into().unwrap());
        let y = i32::from_le_bytes(data[4..8].try_into().unwrap());
        let scale = u32::from_le_bytes(data[8..12].try_into().unwrap());

        world
            .insert(entity, Transform { x, y, scale })
            .map_err(|e| CodecError::DecodeFailed {
                schema: "core.transform".into(),
                reason: format!("{e}"),
            })?;
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 1. MIGRATION CHAINS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn multi_step_migration_chain() {
    let mut migrations = MigrationRegistry::new();
    let schema = SchemaId::new("core.transform");

    // v1 -> v2: adds y=0 (v1 had only x)
    migrations
        .register_step(
            schema.clone(),
            1,
            2,
            Box::new(|v1_data| {
                let mut v2 = v1_data.to_vec();
                v2.extend_from_slice(&0i32.to_le_bytes()); // default y
                Ok(v2)
            }),
        )
        .unwrap();

    // v2 -> v3: adds scale=100 (v2 had x, y)
    migrations
        .register_step(
            schema.clone(),
            2,
            3,
            Box::new(|v2_data| {
                let mut v3 = v2_data.to_vec();
                v3.extend_from_slice(&100u32.to_le_bytes()); // default scale 100
                Ok(v3)
            }),
        )
        .unwrap();

    // Initial v1 payload: x = 42
    let v1_payload = 42i32.to_le_bytes().to_vec();

    // Migrate v1 to v3
    let v3_payload = migrations.migrate(&schema, 1, 3, v1_payload).unwrap();
    assert_eq!(v3_payload.len(), 12);

    let x = i32::from_le_bytes(v3_payload[0..4].try_into().unwrap());
    let y = i32::from_le_bytes(v3_payload[4..8].try_into().unwrap());
    let scale = u32::from_le_bytes(v3_payload[8..12].try_into().unwrap());

    assert_eq!(x, 42);
    assert_eq!(y, 0);
    assert_eq!(scale, 100);
}

#[test]
fn migration_gap_detected() {
    let mut migrations = MigrationRegistry::new();
    let schema = SchemaId::new("core.transform");

    // Only v1 -> v2 registered, v2 -> v3 is missing!
    migrations
        .register_step(schema.clone(), 1, 2, Box::new(|v1| Ok(v1.to_vec())))
        .unwrap();

    let v1_payload = 10i32.to_le_bytes().to_vec();
    let result = migrations.migrate(&schema, 1, 3, v1_payload);

    assert!(matches!(
        result.unwrap_err(),
        MigrationError::MigrationGap {
            from: 2,
            target: 3,
            ..
        }
    ));
}

// ═══════════════════════════════════════════════════════════════════
// 2. TRANSACTIONAL SCENE LOADER & OPAQUE RECORD PRESERVATION
// ═══════════════════════════════════════════════════════════════════

#[test]
fn scene_loader_remapping_and_opaque_preservation() {
    let mut world = World::new();
    world
        .component_registry_mut()
        .register::<Transform>()
        .unwrap();

    let mut schemas = SchemaRegistry::new();
    schemas
        .register(SchemaId::new("core.transform"), Box::new(TransformCodec))
        .unwrap();

    let mut migrations = MigrationRegistry::new();
    migrations
        .register_step(
            SchemaId::new("core.transform"),
            1,
            2,
            Box::new(|v1| {
                let mut out = v1.to_vec();
                out.extend_from_slice(&50i32.to_le_bytes());
                Ok(out)
            }),
        )
        .unwrap();
    migrations
        .register_step(
            SchemaId::new("core.transform"),
            2,
            3,
            Box::new(|v2| {
                let mut out = v2.to_vec();
                out.extend_from_slice(&100u32.to_le_bytes());
                Ok(out)
            }),
        )
        .unwrap();

    // Build scene document with 2 entities:
    // Entity 1: Transform (v1, will be migrated to v3)
    // Entity 2: UnknownSchema (will be preserved as opaque)
    let mut doc = SceneDocument::new();
    let k1 = EntityKey(1001);
    let k2 = EntityKey(1002);

    doc.add_component(
        k1,
        ComponentRecord {
            schema: "core.transform".into(),
            version: 1,
            payload: 10i32.to_le_bytes().to_vec(),
        },
    );

    doc.add_component(
        k2,
        ComponentRecord {
            schema: "future.unknown_plugin".into(),
            version: 99,
            payload: b"future plugin raw binary data".to_vec(),
        },
    );

    // Load scene
    let result = SceneLoader::load_scene(&mut world, &doc, &schemas, &migrations).unwrap();

    // Check entity remapping
    assert_eq!(result.key_to_entity.len(), 2);
    let e1 = result.key_to_entity[&k1];
    let e2 = result.key_to_entity[&k2];

    assert!(world.is_alive(e1));
    assert!(world.is_alive(e2));

    // Check migrated Transform component on e1
    let t = world.get::<Transform>(e1).unwrap();
    assert_eq!(t.x, 10);
    assert_eq!(t.y, 50);
    assert_eq!(t.scale, 100);

    // Check opaque preservation for e2
    assert_eq!(result.preserved_opaque.len(), 1);
    assert_eq!(result.preserved_opaque[0].entity_key, k2);
    assert_eq!(
        result.preserved_opaque[0].record.schema,
        "future.unknown_plugin"
    );
    assert_eq!(
        result.preserved_opaque[0].record.payload,
        b"future plugin raw binary data"
    );
}

// ═══════════════════════════════════════════════════════════════════
// 3. FAIL-CLOSED ROLLBACK ON LOAD FAILURE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn rollback_on_decode_failure() {
    let mut world = World::new();
    world
        .component_registry_mut()
        .register::<Transform>()
        .unwrap();

    let initial_alive_count = 1; // WORLD entity is always alive (index 0)

    let mut schemas = SchemaRegistry::new();
    schemas
        .register(SchemaId::new("core.transform"), Box::new(TransformCodec))
        .unwrap();
    let migrations = MigrationRegistry::new();

    // Corrupt payload (only 2 bytes instead of 12 for v3)
    let mut doc = SceneDocument::new();
    let k1 = EntityKey(101);
    let k2 = EntityKey(102);

    doc.create_entity(k1);
    doc.add_component(
        k2,
        ComponentRecord {
            schema: "core.transform".into(),
            version: 3,
            payload: vec![0xAA, 0xBB], // corrupt!
        },
    );

    let result = SceneLoader::load_scene(&mut world, &doc, &schemas, &migrations);
    assert!(result.is_err(), "load with corrupt payload must fail");

    // Verify rollback: spawned entities were despawned
    // No new entities remain in world
    assert_eq!(
        world.entity_count(),
        initial_alive_count,
        "world must be restored to clean state after load failure"
    );
}

#[test]
fn malformed_scene_document_is_rejected_without_allocating_entities() {
    let mut world = World::new();
    let mut doc = SceneDocument::new();
    doc.entities.push(EntityKey(1));
    doc.entities.push(EntityKey(1));

    let result = SceneLoader::load_scene(
        &mut world,
        &doc,
        &SchemaRegistry::new(),
        &MigrationRegistry::new(),
    );

    assert!(matches!(result, Err(SceneError::InvalidDocument(_))));
    assert_eq!(world.entity_count(), 1, "only WORLD_ENTITY may remain");
}

#[test]
fn duplicate_schema_and_invalid_migration_steps_are_rejected() {
    let mut schemas = SchemaRegistry::new();
    schemas
        .register(SchemaId::new("core.transform"), Box::new(TransformCodec))
        .unwrap();
    assert!(matches!(
        schemas.register(SchemaId::new("core.transform"), Box::new(TransformCodec)),
        Err(CodecError::DuplicateSchema(_))
    ));

    let mut migrations = MigrationRegistry::new();
    let schema = SchemaId::new("core.transform");
    assert!(matches!(
        migrations.register_step(schema.clone(), 2, 2, Box::new(|data| Ok(data.to_vec()))),
        Err(MigrationError::InvalidStep { .. })
    ));
    migrations
        .register_step(schema.clone(), 1, 2, Box::new(|data| Ok(data.to_vec())))
        .unwrap();
    assert!(matches!(
        migrations.register_step(schema, 1, 3, Box::new(|data| Ok(data.to_vec()))),
        Err(MigrationError::DuplicateStep { .. })
    ));
}
