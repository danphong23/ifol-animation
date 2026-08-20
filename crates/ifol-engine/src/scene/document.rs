//! Scene document, stable persistent entity keys, and serialized component records.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

/// Stable identity of a scene session inside a project.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SceneId(String);

impl SceneId {
    /// The default scene identity used by the legacy-free convenience loader.
    pub fn entry() -> Self {
        Self("entry".into())
    }

    /// Creates a non-empty scene identity.
    pub fn new(id: impl Into<String>) -> Option<Self> {
        let id = id.into();
        if id.trim().is_empty() || id.contains('\0') {
            return None;
        }
        Some(Self(id))
    }

    /// Returns the stable scene identity.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SceneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable persistent entity identifier used in serialized scene documents.
///
/// Distinct from runtime `EntityId` (which uses generational indexing in memory).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityKey(pub u64);

impl fmt::Display for EntityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "k{}", self.0)
    }
}

/// A serialized component record associated with an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentRecord {
    /// Schema identifier (e.g. `"core.transform"`).
    pub schema: String,
    /// Schema version of the payload data.
    pub version: u32,
    /// Serialized payload bytes.
    pub payload: Vec<u8>,
}

/// An opaque record preserving unknown component data verbatim across round-trips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpaqueRecord {
    pub entity_key: EntityKey,
    pub record: ComponentRecord,
}

/// Scene document containing persistent entity declarations and component records.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SceneDocument {
    /// List of declared entities.
    pub entities: Vec<EntityKey>,
    /// Component records mapped by entity key.
    pub components: BTreeMap<EntityKey, Vec<ComponentRecord>>,
    /// Opaque records preserved for unrecognized schemas.
    pub opaque_records: Vec<OpaqueRecord>,
}

impl SceneDocument {
    /// Creates an empty scene document.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares a new entity and returns its key.
    pub fn create_entity(&mut self, key: EntityKey) {
        if !self.entities.contains(&key) {
            self.entities.push(key);
            self.components.entry(key).or_default();
        }
    }

    /// Attaches a component record to an entity.
    pub fn add_component(&mut self, key: EntityKey, record: ComponentRecord) {
        self.create_entity(key);
        self.components.entry(key).or_default().push(record);
    }

    /// Attaches an opaque record.
    pub fn add_opaque(&mut self, opaque: OpaqueRecord) {
        self.create_entity(opaque.entity_key);
        self.opaque_records.push(opaque);
    }

    /// Validates the structural invariants required by the scene loader.
    pub fn validate(&self) -> Result<(), String> {
        let declared: BTreeSet<EntityKey> = self.entities.iter().copied().collect();
        if declared.len() != self.entities.len() {
            return Err("duplicate serialized entity key".into());
        }

        for key in self.components.keys() {
            if !declared.contains(key) {
                return Err(format!(
                    "component records reference undeclared entity '{key}'"
                ));
            }
        }

        for (key, records) in &self.components {
            let mut schemas = BTreeSet::new();
            for record in records {
                if record.schema.trim().is_empty() {
                    return Err(format!("entity '{key}' contains an empty schema ID"));
                }
                if !schemas.insert(record.schema.clone()) {
                    return Err(format!(
                        "entity '{key}' contains duplicate component schema '{}'",
                        record.schema
                    ));
                }
            }
        }

        for opaque in &self.opaque_records {
            if !declared.contains(&opaque.entity_key) {
                return Err(format!(
                    "opaque record references undeclared entity '{}'",
                    opaque.entity_key
                ));
            }
            if opaque.record.schema.trim().is_empty() {
                return Err(format!(
                    "opaque record on entity '{}' contains an empty schema ID",
                    opaque.entity_key
                ));
            }
        }

        Ok(())
    }
}
