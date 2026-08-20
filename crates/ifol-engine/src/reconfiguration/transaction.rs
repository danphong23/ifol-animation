//! Dynamic reconfiguration transaction orchestrator.

use crate::package::{PackageId, PackageLock};
use thiserror::Error;

/// Report returned on successful reconfiguration.
#[derive(Debug, Clone)]
pub struct ReconfigurationReport {
    /// Packages successfully added.
    pub added_packages: Vec<PackageId>,
    /// Packages successfully removed.
    pub removed_packages: Vec<PackageId>,
    /// Updated package lock graph.
    pub new_lock: PackageLock,
    /// Engine revision after swap.
    pub revision: u64,
}

/// Errors occurring during dynamic reconfiguration.
#[derive(Debug, Error)]
pub enum ReconfigurationError {
    #[error("package resolution error during reconfiguration: {0}")]
    Resolution(#[from] crate::package::ResolveError),

    #[error("registration transaction error during reconfiguration: {0}")]
    Registration(#[from] crate::registration::TransactionError),

    #[error("reconfiguration failed: {0}")]
    Failed(String),
}
