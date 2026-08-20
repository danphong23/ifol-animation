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

/// Headless composition runtime.
///
/// Created exclusively via [`EngineBuilder::build()`](crate::builder::EngineBuilder::build).
/// Transitions through the lifecycle state machine; see the
/// [lifecycle docs](../docs/01-ownership-and-lifecycle.md) for the
/// full diagram.
pub struct EngineRuntime {
    ecs: ifol_ecs::EcsRuntime,
    command_registry: crate::registration::CommandRegistry,
    provider_manager: crate::provider::ProviderManager,
    package_lock: crate::package::PackageLock,
    project: Option<crate::project::ProjectContainer>,
    schemas: crate::scene::SchemaRegistry,
    migrations: crate::scene::MigrationRegistry,
    state: EngineState,
    revision: u64,
}

impl std::fmt::Debug for EngineRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineRuntime")
            .field("state", &self.state)
            .field("revision", &self.revision)
            .field("command_registry", &self.command_registry)
            .field("provider_manager", &self.provider_manager)
            .field("package_lock", &self.package_lock)
            .field("has_project", &self.project.is_some())
            .field("schema_count", &self.schemas.len())
            .finish()
    }
}

impl EngineRuntime {
    // ── Construction (crate-internal) ───────────────────────────────

    /// Create a runtime from pre-validated parts.
    /// Only called by `EngineBuilder::build()`.
    pub(crate) fn from_parts(
        ecs: ifol_ecs::EcsRuntime,
        command_registry: crate::registration::CommandRegistry,
        provider_manager: crate::provider::ProviderManager,
        package_lock: crate::package::PackageLock,
        project: Option<crate::project::ProjectContainer>,
        schemas: crate::scene::SchemaRegistry,
        migrations: crate::scene::MigrationRegistry,
    ) -> Self {
        Self {
            ecs,
            command_registry,
            provider_manager,
            package_lock,
            project,
            schemas,
            migrations,
            state: EngineState::Ready,
            revision: 0,
        }
    }

    // ── Queries (always valid except ShuttingDown) ──────────────────

    /// Returns a reference to the provider manager.
    #[inline]
    pub fn provider_manager(&self) -> &crate::provider::ProviderManager {
        &self.provider_manager
    }

    /// Returns a reference to the command/query/event registry.
    #[inline]
    pub fn command_registry(&self) -> &crate::registration::CommandRegistry {
        &self.command_registry
    }

    /// Returns the current lifecycle state.
    #[inline]
    pub fn state(&self) -> EngineState {
        self.state
    }

    /// Returns the current engine revision counter.
    #[inline]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns the resolved package lock used to construct this runtime.
    #[inline]
    pub fn package_lock(&self) -> &crate::package::PackageLock {
        &self.package_lock
    }

    /// Returns the project session, when this runtime was built for a project.
    #[inline]
    pub fn project(&self) -> Option<&crate::project::ProjectContainer> {
        self.project.as_ref()
    }

    /// Returns the immutable registry of package-owned component schemas.
    #[inline]
    pub fn schema_registry(&self) -> &crate::scene::SchemaRegistry {
        &self.schemas
    }

    /// Returns the immutable registry of package-owned scene migrations.
    #[inline]
    pub fn migration_registry(&self) -> &crate::scene::MigrationRegistry {
        &self.migrations
    }

    /// Loads one validated scene document into the ECS world atomically.
    pub fn load_scene(
        &mut self,
        document: &crate::scene::SceneDocument,
    ) -> Result<crate::scene::SceneLoadResult, EngineError> {
        self.require_state(EngineState::Ready, "load_scene")?;
        let result = crate::scene::SceneLoader::load_scene(
            self.ecs.world_mut(),
            document,
            &self.schemas,
            &self.migrations,
        )?;
        self.revision = self.revision.wrapping_add(1);
        Ok(result)
    }

    // ── Dynamic Reconfiguration ─────────────────────────────────────

    /// Dynamically reconfigures the active packages and schedule using an atomic stage-and-swap transaction.
    ///
    /// If registration or schedule compilation fails, the live runtime remains completely
    /// untouched and in the `Ready` state.
    pub fn reconfigure(
        &mut self,
        request: crate::reconfiguration::ReconfigurationRequest,
    ) -> Result<crate::reconfiguration::ReconfigurationReport, EngineError> {
        self.require_state(EngineState::Ready, "reconfigure")?;

        let crate::reconfiguration::ReconfigurationRequest {
            transaction,
            command_registry,
            schemas,
            migrations,
            provider_manager,
            package_lock: new_lock,
            added_packages,
            removed_packages,
        } = request;

        // 1. Build staging runtime
        let staging_ecs = ifol_ecs::EcsRuntime::new();
        let staging_cmd_reg = command_registry;

        // 2. Commit transaction onto staging ECS
        let (
            mut staging_ecs,
            staging_cmd_reg,
            staging_schemas,
            staging_migrations,
            mut provider_manager,
        ) = transaction.commit(
            staging_ecs,
            staging_cmd_reg,
            schemas,
            migrations,
            provider_manager,
        )?;
        provider_manager.init_all(&mut staging_ecs)?;

        // 3. Atomic swap
        self.ecs = staging_ecs;
        self.command_registry = staging_cmd_reg;
        self.schemas = staging_schemas;
        self.migrations = staging_migrations;
        self.provider_manager = provider_manager;
        self.package_lock = new_lock.clone();
        self.revision = self.revision.wrapping_add(1);

        Ok(crate::reconfiguration::ReconfigurationReport {
            added_packages,
            removed_packages,
            new_lock,
            revision: self.revision,
        })
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
