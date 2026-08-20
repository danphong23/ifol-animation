//! Schema version migration chains for backward compatibility.
//!
//! Enables upgrading older serialized component records (e.g. V1) to the
//! latest version (e.g. V3) by executing intermediate migration steps.

use crate::scene::schema::SchemaId;
use std::collections::BTreeMap;
use thiserror::Error;

/// Errors during migration execution.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum MigrationError {
    #[error("migration gap: no path to migrate schema '{schema}' from version {from} to {target}")]
    MigrationGap {
        schema: String,
        from: u32,
        target: u32,
    },

    #[error("migration step failed for schema '{schema}' (v{from} -> v{to}): {reason}")]
    StepFailed {
        schema: String,
        from: u32,
        to: u32,
        reason: String,
    },

    #[error("migration step already exists for schema '{schema}' from version {from}")]
    DuplicateStep { schema: String, from: u32 },

    #[error("invalid migration step for schema '{schema}': {from} -> {to}")]
    InvalidStep { schema: String, from: u32, to: u32 },
}

/// Type alias for a single migration function from version N to version N+1.
pub type MigrationFn = Box<dyn Fn(&[u8]) -> Result<Vec<u8>, String> + Send + Sync>;

/// Registry storing migration chains for schemas.
#[derive(Default)]
pub struct MigrationRegistry {
    // (SchemaId, from_version) -> (to_version, MigrationFn)
    steps: BTreeMap<(SchemaId, u32), (u32, MigrationFn)>,
}

impl MigrationRegistry {
    pub fn new() -> Self {
        Self {
            steps: BTreeMap::new(),
        }
    }

    /// Registers a single migration step from `from_version` to `to_version`.
    pub fn register_step(
        &mut self,
        schema: SchemaId,
        from_version: u32,
        to_version: u32,
        step: MigrationFn,
    ) -> Result<(), MigrationError> {
        if from_version >= to_version {
            return Err(MigrationError::InvalidStep {
                schema: schema.to_string(),
                from: from_version,
                to: to_version,
            });
        }
        if self.steps.contains_key(&(schema.clone(), from_version)) {
            return Err(MigrationError::DuplicateStep {
                schema: schema.to_string(),
                from: from_version,
            });
        }
        self.steps
            .insert((schema, from_version), (to_version, step));
        Ok(())
    }

    /// Migrates a payload from `current_version` to `target_version` using a chain of steps.
    pub fn migrate(
        &self,
        schema: &SchemaId,
        mut current_version: u32,
        target_version: u32,
        mut payload: Vec<u8>,
    ) -> Result<Vec<u8>, MigrationError> {
        if current_version == target_version {
            return Ok(payload);
        }

        while current_version < target_version {
            let Some((next_ver, step_fn)) = self.steps.get(&(schema.clone(), current_version))
            else {
                return Err(MigrationError::MigrationGap {
                    schema: schema.to_string(),
                    from: current_version,
                    target: target_version,
                });
            };

            if *next_ver <= current_version || *next_ver > target_version {
                return Err(MigrationError::MigrationGap {
                    schema: schema.to_string(),
                    from: current_version,
                    target: target_version,
                });
            }

            payload = step_fn(&payload).map_err(|reason| MigrationError::StepFailed {
                schema: schema.to_string(),
                from: current_version,
                to: *next_ver,
                reason,
            })?;

            current_version = *next_ver;
        }

        Ok(payload)
    }
}
