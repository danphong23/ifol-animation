//! Typed, fail-closed error hierarchy for `ifol-engine`.
//!
//! Every error variant carries structured identity and operation context.
//! Error discrimination never relies on string parsing; human-readable
//! messages are for diagnostics only.

use thiserror::Error;

/// Top-level engine error returned by all public API methods.
///
/// Variants are grouped by the subsystem/operation that produced them,
/// matching the taxonomy in the engine design docs.
#[derive(Debug, Error)]
pub enum EngineError {
    // ── Lifecycle / state machine ───────────────────────────────────────
    /// A method was called in a state where it is not valid.
    #[error("invalid engine state: expected {expected}, actual {actual}")]
    InvalidState {
        expected: &'static str,
        actual: &'static str,
    },

    /// The engine has already been shut down; no further operations allowed.
    #[error("engine has been shut down")]
    AlreadyShutdown,

    /// A step is already in progress (reentrancy rejected).
    #[error("concurrent or reentrant step rejected")]
    StepInProgress,

    // ── ECS delegation ─────────────────────────────────────────────────
    /// An error propagated from `ifol-ecs` during compile or execution.
    #[error("ECS error: {0}")]
    Ecs(#[from] ifol_ecs::EcsError),

    // ── Build / configuration / registration ───────────────────────────
    /// Engine build failed during validation or compilation.
    #[error("engine build failed: {reason}")]
    BuildFailed { reason: String },

    /// Registration transaction error.
    #[error("registration error: {0}")]
    Registration(#[from] crate::registration::TransactionError),

    /// Package dependency resolution error.
    #[error("package resolution error: {0}")]
    Resolution(#[from] crate::package::ResolveError),

    /// Resource provider error.
    #[error("resource provider error: {0}")]
    Provider(#[from] crate::provider::ProviderError),

    /// Project container or namespace error.
    #[error("project error: {0}")]
    Project(#[from] crate::project::ProjectError),
}
