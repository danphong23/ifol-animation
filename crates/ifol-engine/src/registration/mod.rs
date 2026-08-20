//! Transactional registration system.
//!
//! Provides `RegistrationContext` for packages to declare contributions
//! (components, systems, phases, commands, queries, events) into a
//! staging area. The engine validates the staged contributions atomically
//! and either commits all or discards all — no partial activation.

mod command_registry;
mod context;
mod staging;
mod transaction;

pub use command_registry::{
    CommandHandler, CommandId, CommandReceipt, CommandRegistry, EventDescriptor, EventId,
    QueryHandler, QueryId,
};
pub use context::RegistrationContext;
pub use staging::{StagedContribution, StagedPhaseEdge, StagedSystem};
pub use transaction::{RegistrationTransaction, TransactionError};
