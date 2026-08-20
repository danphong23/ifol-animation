//! `RegistrationContext` — the controlled API surface exposed to packages.
//!
//! Packages receive a `&mut RegistrationContext` and can only contribute
//! through its methods. They never receive `&mut World`, `&mut EcsRuntime`,
//! or any internal registry.

use crate::package::PackageId;
use crate::registration::staging::{StagedContribution, StagedPhaseEdge, StagedSystem};
use ifol_ecs::{AccessDescriptor, Component, PhaseId, RunCondition, SystemContext, SystemError};

/// Controlled registration API exposed to packages.
///
/// Collects contributions into a staging area. The contributions are
/// validated and committed atomically by
/// [`RegistrationTransaction`](crate::registration::RegistrationTransaction).
///
/// The context does **not** give access to `&mut World`, internal
/// registries, or subsystem implementations.
pub struct RegistrationContext {
    owner: PackageId,
    staging: StagedContribution,
}

impl RegistrationContext {
    /// Creates a new context for the given package.
    pub(crate) fn new(owner: PackageId) -> Self {
        Self {
            owner,
            staging: StagedContribution::default(),
        }
    }

    /// Registers a component type.
    pub fn register_component<T: Component>(&mut self) {
        self.staging
            .component_registrations
            .push(Box::new(|ecs: &mut ifol_ecs::EcsRuntime| {
                ecs.register_component::<T>()?;
                Ok(())
            }));
    }

    /// Registers a world singleton component type.
    pub fn register_world_singleton<T: Component>(&mut self) {
        self.staging
            .singleton_registrations
            .push(Box::new(|ecs: &mut ifol_ecs::EcsRuntime| {
                ecs.register_world_singleton::<T>()?;
                Ok(())
            }));
    }

    /// Registers an execution phase.
    pub fn register_phase(&mut self, id: PhaseId) {
        self.staging.phases.push(id);
    }

    /// Adds a directional phase dependency edge.
    pub fn add_phase_edge(&mut self, from: PhaseId, to: PhaseId) {
        self.staging.phase_edges.push(StagedPhaseEdge { from, to });
    }

    /// Registers a closure-based system and binds it to a phase.
    pub fn register_system<F>(
        &mut self,
        name: impl Into<String>,
        phase: PhaseId,
        f: F,
        access: AccessDescriptor,
        conditions: Vec<RunCondition>,
    ) where
        F: FnMut(&mut SystemContext<'_>) -> Result<(), SystemError> + 'static + Send + Sync,
    {
        self.staging.systems.push(StagedSystem {
            name: name.into(),
            system: Box::new(f),
            access,
            conditions,
            phase,
            owner: self.owner.clone(),
        });
    }

    /// Registers a typed command handler.
    pub fn register_command(
        &mut self,
        id: crate::registration::command_registry::CommandId,
        handler: crate::registration::command_registry::CommandHandler,
    ) {
        self.staging.commands.push((id, handler));
    }

    /// Registers a typed query handler.
    pub fn register_query(
        &mut self,
        id: crate::registration::command_registry::QueryId,
        handler: crate::registration::command_registry::QueryHandler,
    ) {
        self.staging.queries.push((id, handler));
    }

    /// Registers an event descriptor.
    pub fn register_event(
        &mut self,
        descriptor: crate::registration::command_registry::EventDescriptor,
    ) {
        self.staging.events.push(descriptor);
    }

    /// Returns the owner package ID.
    pub fn owner(&self) -> &PackageId {
        &self.owner
    }

    /// Consumes the context and returns the staged contributions.
    pub(crate) fn into_staging(self) -> StagedContribution {
        self.staging
    }
}
