//! Engine state machine and lifecycle types.
//!
//! `EngineState` is the finite-state machine that guards every public
//! method on [`EngineRuntime`](crate::runtime::EngineRuntime).
//! A method call in a wrong state returns `EngineError::InvalidState`
//! with the expected vs. actual state names — it never panics and never
//! silently no-ops.

/// Finite states of the engine runtime lifecycle.
///
/// ```text
/// [*] → Building → Ready ⇄ Stepping
///                  Ready → Reconfiguring → Ready
///                  Ready → ShuttingDown → [*]
///                  Faulted → ShuttingDown → [*]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    /// Builder is accumulating packages/project/config before `build()`.
    Building,
    /// Runtime is compiled and idle — ready to accept `step()` or `reconfigure()`.
    Ready,
    /// A `step()` is currently executing; reentrant calls are rejected.
    Stepping,
    /// An unrecoverable invariant or service failure occurred during `step()`.
    Faulted,
    /// `shutdown()` has been called; no further operations are accepted.
    ShuttingDown,
}

impl EngineState {
    /// Returns the human-readable label for diagnostics.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Building => "Building",
            Self::Ready => "Ready",
            Self::Stepping => "Stepping",
            Self::Faulted => "Faulted",
            Self::ShuttingDown => "ShuttingDown",
        }
    }
}

impl std::fmt::Display for EngineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}
