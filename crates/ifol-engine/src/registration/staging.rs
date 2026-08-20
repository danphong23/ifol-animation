use crate::package::PackageId;
use ifol_ecs::{AccessDescriptor, PhaseId, RunCondition, SystemContext, SystemError};

/// Type alias for staged system execution closure.
pub type StagedSystemFn =
    Box<dyn FnMut(&mut SystemContext<'_>) -> Result<(), SystemError> + Send + Sync>;

/// Type alias for staged ECS component or singleton registration callback.
pub type StagedRegistrationFn =
    Box<dyn FnOnce(&mut ifol_ecs::EcsRuntime) -> Result<(), ifol_ecs::EcsError>>;

/// A phase edge to be added during registration.
#[derive(Debug, Clone)]
pub struct StagedPhaseEdge {
    pub from: PhaseId,
    pub to: PhaseId,
}

/// A staged system to be registered and bound to a phase.
pub struct StagedSystem {
    pub name: String,
    pub system: StagedSystemFn,
    pub access: AccessDescriptor,
    pub conditions: Vec<RunCondition>,
    pub phase: PhaseId,
    pub owner: PackageId,
}

impl std::fmt::Debug for StagedSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StagedSystem")
            .field("name", &self.name)
            .field("phase", &self.phase)
            .field("owner", &self.owner)
            .finish()
    }
}

/// All staged contributions from a single package.
#[derive(Default)]
pub struct StagedContribution {
    /// Component types to register (as type-erased registration closures).
    pub component_registrations: Vec<StagedRegistrationFn>,
    /// World singleton types to register.
    pub singleton_registrations: Vec<StagedRegistrationFn>,
    /// Phases to register.
    pub phases: Vec<PhaseId>,
    /// Phase edges to add.
    pub phase_edges: Vec<StagedPhaseEdge>,
    /// Systems to register and bind.
    pub systems: Vec<StagedSystem>,
    /// Commands to register.
    pub commands: Vec<(
        crate::registration::command_registry::CommandId,
        crate::registration::command_registry::CommandHandler,
    )>,
    /// Queries to register.
    pub queries: Vec<(
        crate::registration::command_registry::QueryId,
        crate::registration::command_registry::QueryHandler,
    )>,
    /// Events to register.
    pub events: Vec<crate::registration::command_registry::EventDescriptor>,
}

impl std::fmt::Debug for StagedContribution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StagedContribution")
            .field("component_count", &self.component_registrations.len())
            .field("singleton_count", &self.singleton_registrations.len())
            .field("phase_count", &self.phases.len())
            .field("phase_edge_count", &self.phase_edges.len())
            .field("system_count", &self.systems.len())
            .field("command_count", &self.commands.len())
            .field("query_count", &self.queries.len())
            .field("event_count", &self.events.len())
            .finish()
    }
}
