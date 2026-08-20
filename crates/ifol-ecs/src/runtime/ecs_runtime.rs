use crate::error::{EcsError, SystemError};
use crate::registry::{ComponentId, PhaseId, PhaseRegistry, SystemId, SystemRegistry};
use crate::report::{ExecutionPolicy, RunReport};
use crate::schedule::CompiledSchedule;
use crate::storage::Component;
use crate::system::{
    AccessDescriptor, Commands, FunctionSystem, RunCondition, System, SystemContext,
};
use crate::world::World;

/// Central composition root owning World, Registries, Compiled Schedule, and Query Plan Cache.
pub struct EcsRuntime {
    pub(crate) world: World,
    pub(crate) phase_registry: PhaseRegistry,
    pub(crate) system_registry: SystemRegistry,
    pub(crate) compiled_schedule: Option<CompiledSchedule>,
    pub(crate) commands: Commands,
    pub(crate) execution_counter: u64,
    pub(crate) execution_policy: ExecutionPolicy,
}

impl EcsRuntime {
    /// Creates a new empty `EcsRuntime`.
    pub fn new() -> Self {
        Self {
            world: World::new(),
            phase_registry: PhaseRegistry::new(),
            system_registry: SystemRegistry::new(),
            compiled_schedule: None,
            commands: Commands::new(),
            execution_counter: 0,
            execution_policy: ExecutionPolicy::default(),
        }
    }

    /// Registers a component type `T` in the runtime's component registry.
    pub fn register_component<T: Component>(&mut self) -> Result<ComponentId, EcsError> {
        self.world.component_registry_mut().register::<T>()
    }

    /// Registers a component type `T` designated as a world singleton on `WORLD_ENTITY`.
    pub fn register_world_singleton<T: Component>(&mut self) -> Result<ComponentId, EcsError> {
        self.world
            .component_registry_mut()
            .register_world_singleton::<T>()
    }

    /// Registers an execution phase into the runtime's phase registry.
    pub fn register_phase(&mut self, id: PhaseId) -> Result<(), EcsError> {
        self.phase_registry.register_phase(id)
    }

    /// Registers a system into the runtime's system registry.
    pub fn register_system<S: System>(
        &mut self,
        name: impl Into<String>,
        system: S,
        access: AccessDescriptor,
        conditions: Vec<RunCondition>,
    ) -> Result<SystemId, EcsError> {
        self.system_registry
            .register(name.into(), Box::new(system), access, conditions)
    }

    /// Registers a closure-based system into the runtime.
    pub fn register_function_system<F>(
        &mut self,
        name: impl Into<String>,
        f: F,
        access: AccessDescriptor,
        conditions: Vec<RunCondition>,
    ) -> Result<SystemId, EcsError>
    where
        F: FnMut(&mut SystemContext<'_>) -> Result<(), SystemError> + 'static + Send + Sync,
    {
        self.register_system(name, FunctionSystem::new(f), access, conditions)
    }

    /// Attaches a registered system to the specified phase.
    pub fn attach_system(&mut self, phase: &PhaseId, system: SystemId) -> Result<(), EcsError> {
        if self.system_registry.get(system).is_none() {
            return Err(EcsError::SystemNotFound(format!("binding {system:?}")));
        }
        self.phase_registry.attach_system(phase, system)
    }

    /// Adds a directional phase dependency edge: `from` must execute BEFORE `to`.
    pub fn add_phase_edge(&mut self, from: &PhaseId, to: &PhaseId) -> Result<(), EcsError> {
        self.phase_registry.add_phase_edge(from, to)
    }

    /// Validates all registrations, detects cycles, and compiles the phase DAG into an owned schedule.
    pub fn compile(&mut self) -> Result<(), EcsError> {
        // Never retain an old executable plan after a failed rebuild.
        self.compiled_schedule = None;
        self.system_registry
            .validate_components(self.world.component_registry())?;
        let schedule = CompiledSchedule::compile(&self.phase_registry, &self.system_registry)?;
        self.compiled_schedule = Some(schedule);
        Ok(())
    }

    /// Executes one execution pass over the world.
    ///
    /// If the phase graph changed since last compilation, returns `Err(EcsError::ScheduleNotCompiled)`.
    pub fn run_once(&mut self) -> Result<RunReport, EcsError> {
        let Some(schedule) = &mut self.compiled_schedule else {
            return Err(EcsError::ScheduleNotCompiled);
        };

        if schedule.graph_revision() != self.phase_registry.revision() {
            return Err(EcsError::ScheduleNotCompiled);
        }

        self.execution_counter = self.execution_counter.wrapping_add(1);
        let execution_rev = self.execution_counter;

        schedule.run_pass(
            &mut self.world,
            &mut self.system_registry,
            &mut self.commands,
            execution_rev,
            self.execution_policy,
        )
    }

    /// Returns the current system-error execution policy.
    #[inline]
    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.execution_policy
    }

    /// Sets the system-error execution policy for future passes.
    #[inline]
    pub fn set_execution_policy(&mut self, policy: ExecutionPolicy) {
        self.execution_policy = policy;
    }
}

impl Default for EcsRuntime {
    fn default() -> Self {
        Self::new()
    }
}
