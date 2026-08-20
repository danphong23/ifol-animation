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
    namespaces: crate::project::NamespaceRegistry,
    active_scene: Option<crate::scene::SceneId>,
    active_scene_entities: BTreeSet<ifol_ecs::entity::EntityId>,
    state: EngineState,
    revision: u64,
}

pub(crate) struct RuntimeParts {
    pub ecs: ifol_ecs::EcsRuntime,
    pub command_registry: crate::registration::CommandRegistry,
    pub provider_manager: crate::provider::ProviderManager,
    pub package_lock: crate::package::PackageLock,
    pub project: Option<crate::project::ProjectContainer>,
    pub schemas: crate::scene::SchemaRegistry,
    pub migrations: crate::scene::MigrationRegistry,
    pub namespaces: crate::project::NamespaceRegistry,
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
            project: parts.project,
            schemas: parts.schemas,
            migrations: parts.migrations,
            namespaces: parts.namespaces,
            active_scene: None,
            active_scene_entities: BTreeSet::new(),
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

    /// Returns the project namespace claims active in this runtime.
    #[inline]
    pub fn namespace_registry(&self) -> &crate::project::NamespaceRegistry {
        &self.namespaces
    }

    /// Returns the currently active scene identity, if a scene was loaded.
    pub fn active_scene(&self) -> Option<&crate::scene::SceneId> {
        self.active_scene.as_ref()
    }

    /// Returns the number of entities owned by the active scene.
    pub fn active_scene_entity_count(&self) -> usize {
        self.active_scene_entities.len()
    }

    /// Loads one validated scene document into the ECS world atomically.
    pub fn load_scene(
        &mut self,
        document: &crate::scene::SceneDocument,
    ) -> Result<crate::scene::SceneLoadResult, EngineError> {
        self.load_scene_as(crate::scene::SceneId::entry(), document)
    }

    /// Loads a scene and replaces the previous active scene after the new
    /// document has loaded successfully. Persistent `WORLD_ENTITY` resources
    /// remain untouched.
    pub fn load_scene_as(
        &mut self,
        scene_id: crate::scene::SceneId,
        document: &crate::scene::SceneDocument,
    ) -> Result<crate::scene::SceneLoadResult, EngineError> {
        self.require_state(EngineState::Ready, "load_scene")?;
        let result = crate::scene::SceneLoader::load_scene(
            self.ecs.world_mut(),
            document,
            &self.schemas,
            &self.migrations,
        )?;

        let new_entities: BTreeSet<_> = result.key_to_entity.values().copied().collect();
        for entity in self.active_scene_entities.iter().copied() {
            if let Err(error) = self.ecs.despawn(entity) {
                self.state = EngineState::Faulted;
                return Err(EngineError::Ecs(error));
            }
        }
        self.active_scene = Some(scene_id.clone());
        self.active_scene_entities = new_entities;
        self.revision = self.revision.wrapping_add(1);
        Ok(crate::scene::SceneLoadResult {
            scene_id: Some(scene_id),
            ..result
        })
    }

    /// Removes the active scene while preserving packages, registrations and
    /// world singleton resources.
    pub fn clear_scene(&mut self) -> Result<bool, EngineError> {
        self.require_state(EngineState::Ready, "clear_scene")?;
        let had_scene = self.active_scene.is_some();
        for entity in self.active_scene_entities.iter().copied() {
            self.ecs.despawn(entity)?;
        }
        self.active_scene = None;
        self.active_scene_entities.clear();
        if had_scene {
            self.revision = self.revision.wrapping_add(1);
        }
        Ok(had_scene)
    }

    // ── Dynamic Reconfiguration ─────────────────────────────────────

    /// Dynamically reconfigures the active packages and schedule using an atomic stage-and-swap transaction.
    ///
    /// Registration, schedule compilation, or candidate-provider initialization
    /// failures leave the live runtime untouched and in `Ready`. If an active
    /// provider teardown fails, external side effects cannot be rolled back;
    /// the runtime transitions to `Faulted` and refuses further stepping.
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
            namespaces,
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
            staging_namespaces,
        ) = transaction.commit(
            staging_ecs,
            staging_cmd_reg,
            schemas,
            migrations,
            provider_manager,
            namespaces,
        )?;
        provider_manager.init_all(&mut staging_ecs)?;

        // Provider teardown is the external side-effect boundary. It cannot be
        // rolled back if a provider itself reports failure, so fault the
        // runtime and refuse further stepping instead of publishing a partial
        // replacement.
        if let Err(error) = self.provider_manager.teardown_all(&mut self.ecs) {
            let _ = provider_manager.teardown_all(&mut staging_ecs);
            self.state = EngineState::Faulted;
            return Err(EngineError::Provider(error));
        }

        // 3. Swap only after all fallible staging and old-provider teardown.
        self.ecs = staging_ecs;
        self.command_registry = staging_cmd_reg;
        self.schemas = staging_schemas;
        self.migrations = staging_migrations;
        self.namespaces = staging_namespaces;
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
