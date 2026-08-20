//! `EngineBuilder` — the construction API for `EngineRuntime`.
//!
//! The builder accumulates configuration then performs atomic
//! validation, commit and compilation before returning a ready
//! runtime. If any step fails, the builder returns a typed
//! `EngineError` and no partial runtime is leaked.

use crate::error::EngineError;
use crate::runtime::EngineRuntime;
use crate::state::EngineState;

/// Fluent builder for constructing an [`EngineRuntime`].
///
/// # Contract
///
/// - The builder starts in the `Building` state.
/// - `build()` validates all accumulated configuration, compiles the
///   ECS schedule, and transitions to `Ready`.
/// - On failure, `build()` returns a typed `EngineError` and the
///   builder can be retried (after fixing the issue) or dropped.
///
/// # Example
///
/// ```rust
/// use ifol_engine::EngineBuilder;
///
/// let engine = EngineBuilder::new().build().unwrap();
/// assert_eq!(engine.state(), ifol_engine::EngineState::Ready);
/// ```
pub struct EngineBuilder {
    _state: EngineState,
}

impl EngineBuilder {
    /// Creates a new builder in the `Building` state.
    pub fn new() -> Self {
        Self {
            _state: EngineState::Building,
        }
    }

    /// Validates configuration, compiles the ECS schedule, and returns
    /// a ready `EngineRuntime`.
    ///
    /// An empty builder (no packages, no project) is valid and produces
    /// a runtime with an empty ECS world that can step deterministically.
    pub fn build(self) -> Result<EngineRuntime, EngineError> {
        let mut ecs = ifol_ecs::EcsRuntime::new();
        ecs.compile()?;

        Ok(EngineRuntime::from_parts(ecs))
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
