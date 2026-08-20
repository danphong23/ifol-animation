use crate::scene::document::{EntityKey, OpaqueRecord, SceneDocument};
use crate::scene::migration::{MigrationError, MigrationRegistry};
use crate::scene::schema::{CodecError, SchemaId, SchemaRegistry};
use ifol_ecs::entity::EntityId;
use ifol_ecs::world::World;
use std::collections::BTreeMap;
use thiserror::Error;

/// Errors during scene loading or saving.
#[derive(Debug, Error)]
pub enum SceneError {
    #[error("codec error: {0}")]
    Codec(#[from] CodecError),

    #[error("migration error: {0}")]
    Migration(#[from] MigrationError),

    #[error("ECS error: {0}")]
    Ecs(#[from] ifol_ecs::EcsError),

    #[error("scene load failed on entity '{entity_key}': {reason}")]
    LoadFailed {
        entity_key: EntityKey,
        reason: String,
    },
}

/// Result of loading a scene into the ECS World.
#[derive(Debug, Clone)]
pub struct SceneLoadResult {
    /// Mapping from persistent `EntityKey` to runtime `EntityId`.
    pub key_to_entity: BTreeMap<EntityKey, EntityId>,
    /// Opaque records preserved for unrecognized schemas.
    pub preserved_opaque: Vec<OpaqueRecord>,
}

/// Transactional scene loader.
pub struct SceneLoader;

impl SceneLoader {
    /// Loads a `SceneDocument` into an ECS `World` atomically.
    ///
    /// # Fail-closed rollback
    ///
    /// If any entity, migration, or component decode fails, all entities
    /// spawned during this load operation are despawned before returning the error.
    pub fn load_scene(
        world: &mut World,
        doc: &SceneDocument,
        schemas: &SchemaRegistry,
        migrations: &MigrationRegistry,
    ) -> Result<SceneLoadResult, SceneError> {
        let mut spawned_entities = Vec::new();
        let mut key_to_entity = BTreeMap::new();
        let mut preserved_opaque = doc.opaque_records.clone();

        // Helper for cleanup on error
        let cleanup = |world: &mut World, spawned: &[EntityId]| {
            for &e in spawned {
                let _ = world.despawn(e);
            }
        };

        // 1. Spawn all entities and record mapping
        for &key in &doc.entities {
            let entity_id = world.spawn();
            spawned_entities.push(entity_id);
            key_to_entity.insert(key, entity_id);
        }

        // 2. Attach components to entities
        for (&key, records) in &doc.components {
            let entity_id = key_to_entity[&key];

            for record in records {
                let schema_id = SchemaId::new(&record.schema);

                if let Some(codec) = schemas.get(&schema_id) {
                    let target_version = codec.current_version();

                    // Apply migration chain if record is older
                    let payload = if record.version < target_version {
                        match migrations.migrate(
                            &schema_id,
                            record.version,
                            target_version,
                            record.payload.clone(),
                        ) {
                            Ok(migrated) => migrated,
                            Err(e) => {
                                cleanup(world, &spawned_entities);
                                return Err(SceneError::Migration(e));
                            }
                        }
                    } else if record.version > target_version {
                        cleanup(world, &spawned_entities);
                        return Err(SceneError::Codec(CodecError::UnsupportedVersion {
                            schema: record.schema.clone(),
                            version: record.version,
                            current: target_version,
                        }));
                    } else {
                        record.payload.clone()
                    };

                    // Decode and insert component into ECS
                    if let Err(e) = codec.decode_and_insert(world, entity_id, &payload) {
                        cleanup(world, &spawned_entities);
                        return Err(SceneError::Codec(e));
                    }
                } else {
                    // Unknown schema -> preserve as opaque record without data loss
                    preserved_opaque.push(OpaqueRecord {
                        entity_key: key,
                        record: record.clone(),
                    });
                }
            }
        }

        Ok(SceneLoadResult {
            key_to_entity,
            preserved_opaque,
        })
    }
}
