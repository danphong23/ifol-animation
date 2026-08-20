//! Scene documents, component schemas, version migrations, and transactional loading.

mod document;
mod loader;
mod migration;
mod schema;

pub use document::{ComponentRecord, EntityKey, OpaqueRecord, SceneDocument};
pub use loader::{SceneError, SceneLoadResult, SceneLoader};
pub use migration::{MigrationError, MigrationFn, MigrationRegistry};
pub use schema::{CodecError, ComponentCodec, SchemaId, SchemaRegistry};
