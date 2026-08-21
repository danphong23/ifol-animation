//! `EngineRuntime` — the headless composition runtime.
//!
//! Owns a single `EcsRuntime`, a state machine, and revision metadata.
//! All public methods are guarded by the state machine: calling a
//! method in the wrong state returns `EngineError::InvalidState`,
//! never panics, and never silently no-ops.
//!
//! The runtime does **not** own any platform loop, window, surface,
//! timer, sleep, or worker queue. It provides exactly one finite
//! unit of work per `step()` invocation.

use crate::error::EngineError;
use crate::report::{ShutdownReport, StepInput, StepReport};
use crate::state::EngineState;
use std::collections::BTreeSet;

#[path = "runtime/inspection.rs"]
mod inspection;
#[path = "runtime/reconfiguration.rs"]
mod reconfiguration;
#[path = "runtime/scene.rs"]
mod scene;

/// Headless composition runtime.
///
/// Created exclusively via [`EngineBuilder::build()`](crate::builder::EngineBuilder::build).
/// Transitions through the lifecycle state machine; see the
/// [lifecycle docs](../docs/01-ownership-and-lifecycle.md) for the
/// full diagram.
pub struct EngineRuntime {
    pub(crate) ecs: ifol_ecs::EcsRuntime,
    pub(crate) command_registry: crate::registration::CommandRegistry,
    pub(crate) provider_manager: crate::provider::ProviderManager,
    pub(crate) package_lock: crate::package::PackageLock,
    pub(crate) schemas: crate::scene::SchemaRegistry,
    pub(crate) migrations: crate::scene::MigrationRegistry,
    pub(crate) namespaces: crate::namespace::NamespaceRegistry,
    pub(crate) active_scene: Option<crate::scene::SceneId>,
    pub(crate) active_scene_entities: BTreeSet<ifol_ecs::entity::EntityId>,
    pub(crate) state: EngineState,
    pub(crate) revision: u64,
}

pub(crate) struct RuntimeParts {
    pub ecs: ifol_ecs::EcsRuntime,
    pub command_registry: crate::registration::CommandRegistry,
    pub provider_manager: crate::provider::ProviderManager,
    pub package_lock: crate::package::PackageLock,
    pub schemas: crate::scene::SchemaRegistry,
    pub migrations: crate::scene::MigrationRegistry,
    pub namespaces: crate::namespace::NamespaceRegistry,
}

impl std::fmt::Debug for EngineRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineRuntime")
            .field("state", &self.state)
            .field("revision", &self.revision)
            .field("command_registry", &self.command_registry)
            .field("provider_manager", &self.provider_manager)
            .field("package_lock", &self.package_lock)
            .field("schema_count", &self.schemas.len())
            .field("namespace_count", &self.namespaces.len())
            .finish()
    }
}

impl EngineRuntime {
    // ── Construction (crate-internal) ───────────────────────────────

    /// Create a runtime from pre-validated parts.
    /// Only called by `EngineBuilder::build()`.
    pub(crate) fn from_parts(parts: RuntimeParts) -> Self {
        Self {
            ecs: parts.ecs,
            command_registry: parts.command_registry,
            provider_manager: parts.provider_manager,
            package_lock: parts.package_lock,
            schemas: parts.schemas,
            migrations: parts.migrations,
            namespaces: parts.namespaces,
            active_scene: None,
            active_scene_entities: BTreeSet::new(),
            state: EngineState::Ready,
            revision: 0,
        }
    }

    // ── Step ────────────────────────────────────────────────────────

    /// Executes one finite step on the ECS world.
    ///
    /// # State requirement
    ///
    /// Must be in `Ready`. Returns `InvalidState` for any other state.
    ///
    /// # Semantics
    ///
    /// 1. Transition to `Stepping` (reject reentrancy).
    /// 2. Run `EcsRuntime::run_once()` exactly once.
    /// 3. Increment revision.
    /// 4. Transition back to `Ready`.
    /// 5. Return `StepReport`.
    ///
    /// If the ECS execution fails with a fail-fast error, the runtime
    /// transitions to `Faulted` instead of `Ready`.
    pub fn step(&mut self, input: StepInput) -> Result<StepReport, EngineError> {
        self.require_state(EngineState::Ready, "step")?;

        // Transition to Stepping (reentrancy guard)
        self.state = EngineState::Stepping;

        // Run ECS exactly once
        let ecs_result = self.ecs.run_once();

        match ecs_result {
            Ok(ecs_report) => {
                self.revision = self.revision.wrapping_add(1);
                self.state = EngineState::Ready;
                Ok(StepReport {
                    correlation_id: input.correlation_id,
                    engine_revision: self.revision,
                    ecs_report,
                })
            }
            Err(e) => {
                // Fail-fast: transition to Faulted
                self.state = EngineState::Faulted;
                Err(EngineError::Ecs(e))
            }
        }
    }

    // ── Shutdown ────────────────────────────────────────────────────

    /// Shuts down the runtime, releasing all resources.
    ///
    /// # State requirement
    ///
    /// Valid from `Ready` or `Faulted`. Returns `InvalidState` for
    /// `Building`, `Stepping`, or if already `ShuttingDown`.
    ///
    /// Shutdown is idempotent in the sense that a second call after
    /// the first successful shutdown returns `AlreadyShutdown`.
    pub fn shutdown(&mut self) -> Result<ShutdownReport, EngineError> {
        match self.state {
            EngineState::ShuttingDown => {
                return Err(EngineError::AlreadyShutdown);
            }
            EngineState::Ready | EngineState::Faulted => {
                // proceed
            }
            other => {
                return Err(EngineError::InvalidState {
                    expected: "Ready or Faulted",
                    actual: other.label(),
                });
            }
        }

        let final_revision = self.revision;
        self.state = EngineState::ShuttingDown;

        // 1. Tear down resource providers in reverse topological order
        let provider_res = self.provider_manager.teardown_all(&mut self.ecs);

        // 2. Tear down ECS
        self.ecs.shutdown();
        self.active_scene = None;
        self.active_scene_entities.clear();

        if let Err(e) = provider_res {
            return Err(EngineError::Provider(e));
        }

        Ok(ShutdownReport {
            final_revision,
            clean: true,
        })
    }

    // ── Internal helpers ───────────────────────────────────────────

    /// Guard: returns `InvalidState` if the current state does not
    /// match `expected`.
    fn require_state(&self, expected: EngineState, method: &str) -> Result<(), EngineError> {
        if self.state != expected {
            return Err(EngineError::InvalidState {
                expected: expected.label(),
                actual: self.state.label(),
            });
        }
        let _ = method; // reserved for future diagnostics
        Ok(())
    }
}
