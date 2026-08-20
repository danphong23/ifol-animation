//! Scene document, stable persistent entity keys, and serialized component records.

use std::collections::BTreeMap;
use std::fmt;

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
}
