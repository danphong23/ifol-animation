//! Component schema definitions, versioning, and serialization codecs.

use ifol_ecs::entity::EntityId;
use ifol_ecs::world::World;
use std::collections::BTreeMap;
use std::fmt;
use thiserror::Error;

/// Schema identifier (e.g. `"core.transform"` or `"vendor.filter"`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaId(String);

impl SchemaId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SchemaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Errors during component encoding/decoding.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum CodecError {
    #[error("schema '{0}' is not registered")]
    SchemaNotFound(String),

    #[error("unsupported schema version {version} for '{schema}' (current: {current})")]
    UnsupportedVersion {
        schema: String,
        version: u32,
        current: u32,
    },

    #[error("decoding error for schema '{schema}': {reason}")]
    DecodeFailed { schema: String, reason: String },

    #[error("encoding error for schema '{schema}': {reason}")]
    EncodeFailed { schema: String, reason: String },
}

/// Codec trait for serializing and deserializing components to/from ECS World.
pub trait ComponentCodec: Send + Sync {
    /// The current schema version supported by this codec.
    fn current_version(&self) -> u32;

    /// Encodes a component from an ECS entity into raw bytes.
    fn encode(&self, world: &World, entity: EntityId) -> Result<Option<Vec<u8>>, CodecError>;

    /// Decodes raw bytes and inserts the component onto the ECS entity.
    fn decode_and_insert(
        &self,
        world: &mut World,
        entity: EntityId,
        data: &[u8],
    ) -> Result<(), CodecError>;
}

/// Registry mapping SchemaId to its active ComponentCodec.
#[derive(Default)]
pub struct SchemaRegistry {
    codecs: BTreeMap<SchemaId, Box<dyn ComponentCodec>>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        Self {
            codecs: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, id: SchemaId, codec: Box<dyn ComponentCodec>) {
        self.codecs.insert(id, codec);
    }

    pub fn get(&self, id: &SchemaId) -> Option<&dyn ComponentCodec> {
        self.codecs.get(id).map(|b| b.as_ref())
    }

    pub fn has_schema(&self, id: &SchemaId) -> bool {
        self.codecs.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.codecs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.codecs.is_empty()
    }
}
