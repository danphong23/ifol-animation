//! Dynamic Reconfiguration subsystem.
//!
//! Enables replacing the active package composition and recompiling the ECS
//! schedule at runtime. The replacement is staged before publication, but it
//! currently creates a fresh ECS runtime; entity/component state is therefore
//! not preserved unless a future explicit state-transfer contract is added.

mod plan;
mod transaction;

pub use plan::ReconfigurationPlan;
pub use transaction::{ReconfigurationError, ReconfigurationReport};

/// Fully prepared candidates for one atomic runtime reconfiguration.
pub struct ReconfigurationRequest {
    /// Staged package contributions to apply to the replacement ECS runtime.
    pub transaction: crate::registration::RegistrationTransaction,
    /// Replacement command/query/event registry.
    pub command_registry: crate::registration::CommandRegistry,
    /// Replacement package-owned schema registry candidate.
    pub schemas: crate::scene::SchemaRegistry,
    /// Replacement package-owned migration registry candidate.
    pub migrations: crate::scene::MigrationRegistry,
    /// Replacement provider manager candidate.
    pub provider_manager: crate::provider::ProviderManager,
    /// Replacement runtime namespace registry candidate.
    pub namespaces: crate::namespace::NamespaceRegistry,
    /// Replacement resolved package lock.
    pub package_lock: crate::package::PackageLock,
    /// Package IDs added by this request, for diagnostics.
    pub added_packages: Vec<crate::package::PackageId>,
    /// Package IDs removed by this request, for diagnostics.
    pub removed_packages: Vec<crate::package::PackageId>,
}
