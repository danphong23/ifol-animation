#![doc = include_str!("../docs/README.md")]

pub mod builder;
pub mod error;
pub mod package;
pub mod registration;
pub mod report;
pub mod runtime;
pub mod state;

// Public re-exports
pub use builder::EngineBuilder;
pub use error::EngineError;
pub use package::{
    PackageDependency, PackageId, PackageLock, PackageManifest, PackageResolver, ResolveError,
    ResolvedPackage, Version, VersionReq,
};
pub use registration::{
    CommandHandler, CommandId, CommandReceipt, CommandRegistry, EventDescriptor, EventId,
    QueryHandler, QueryId, RegistrationContext, RegistrationTransaction, TransactionError,
};
pub use report::{ShutdownReport, StepInput, StepReport};
pub use runtime::EngineRuntime;
pub use state::EngineState;
