#![doc = include_str!("../docs/README.md")]

pub mod builder;
pub mod config;
pub mod error;
pub mod namespace;
pub mod package;
pub mod provider;
pub mod reconfiguration;
pub mod registration;
pub mod report;
pub mod runtime;
pub mod scene;
pub mod state;

// Public re-exports
pub use builder::EngineBuilder;
pub use config::EngineConfig;
pub use error::EngineError;
pub use namespace::{Namespace, NamespaceError, NamespaceRegistry};
pub use package::{
    EnginePackage, PackageDependency, PackageError, PackageId, PackageLock, PackageManifest,
    PackageRegistration, PackageResolver, ResolveError, ResolvedPackage, Version, VersionReq,
};
pub use provider::{ProviderError, ProviderManager, ResourceId, ResourceProvider};
pub use reconfiguration::{
    ReconfigurationError, ReconfigurationPlan, ReconfigurationReport, ReconfigurationRequest,
};
pub use registration::{
    CommandHandler, CommandId, CommandReceipt, CommandRegistry, EventDescriptor, EventId,
    QueryHandler, QueryId, RegistrationContext, RegistrationTransaction, TransactionError,
};
pub use report::{ShutdownReport, StepInput, StepReport};
pub use runtime::EngineRuntime;
pub use scene::{
    CodecError, ComponentCodec, ComponentRecord, EntityKey, MigrationError, MigrationFn,
    MigrationRegistry, OpaqueRecord, SceneDocument, SceneError, SceneId, SceneLoadResult,
    SceneLoader, SchemaId, SchemaRegistry,
};
pub use state::EngineState;
