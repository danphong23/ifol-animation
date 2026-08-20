#![doc = include_str!("../docs/README.md")]

pub mod builder;
pub mod error;
pub mod package;
pub mod project;
pub mod provider;
pub mod reconfiguration;
pub mod registration;
pub mod report;
pub mod runtime;
pub mod scene;
pub mod state;
pub mod testing;

// Public re-exports
pub use builder::EngineBuilder;
pub use error::EngineError;
pub use package::{
    PackageDependency, PackageId, PackageLock, PackageManifest, PackageResolver, ResolveError,
    ResolvedPackage, Version, VersionReq,
};
pub use project::{
    CURRENT_FORMAT_VERSION, MemoryStorage, Namespace, NamespaceError, NamespaceRegistry,
    PackageLockFile, PathSecurity, ProjectContainer, ProjectError, ProjectManifest, ProjectStorage,
    StorageError,
};
pub use provider::{ProviderError, ProviderManager, ResourceId, ResourceProvider};
pub use reconfiguration::{ReconfigurationError, ReconfigurationPlan, ReconfigurationReport};
pub use registration::{
    CommandHandler, CommandId, CommandReceipt, CommandRegistry, EventDescriptor, EventId,
    QueryHandler, QueryId, RegistrationContext, RegistrationTransaction, TransactionError,
};
pub use report::{ShutdownReport, StepInput, StepReport};
pub use runtime::EngineRuntime;
pub use scene::{
    CodecError, ComponentCodec, ComponentRecord, EntityKey, MigrationError, MigrationFn,
    MigrationRegistry, OpaqueRecord, SceneDocument, SceneError, SceneLoadResult, SceneLoader,
    SchemaId, SchemaRegistry,
};
pub use state::EngineState;
