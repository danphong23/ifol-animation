use crate::error::EcsError;
use crate::report::TickReport;
use crate::schedule::dag::{sort_phases, PhaseConfig};
use crate::schedule::phase::PhaseId;
use crate::schedule::system::{FunctionSystem, System};
use crate::world::World;
use std::collections::HashMap;

/// An executable phase containing an ordered list of compiled systems.
pub struct CompiledPhase {
    pub id: PhaseId,
    pub systems: Vec<Box<dyn System>>,
}

impl CompiledPhase {
    /// Returns the identifier of this phase.
    #[inline(always)]
    pub fn id(&self) -> &PhaseId {
        &self.id
    }
}

/// The compiled schedule managing and executing all phases in DAG topological order.
pub struct Schedule {
    phases: Vec<CompiledPhase>,
}

impl Schedule {
    /// Returns a slice of all compiled phases in execution order.
    #[inline(always)]
    pub fn phases(&self) -> &[CompiledPhase] {
        &self.phases
    }

    /// Creates a new `ScheduleBuilder` to configure phases and systems.
    pub fn builder() -> ScheduleBuilder {
        ScheduleBuilder::new()
    }

    /// Executes all scheduled phases and systems in order on the provided `World`.
    ///
    /// Increments the world tick and returns execution diagnostics in `TickReport`.
    pub fn run(&mut self, world: &mut World) -> TickReport {
        let start = std::time::Instant::now();
        let mut systems_executed = 0;

        for phase in &mut self.phases {
            for system in &mut phase.systems {
                system.run(world);
                systems_executed += 1;
            }
        }

        let current_tick = world.increment_tick();

        TickReport {
            duration_us: start.elapsed().as_micros() as u64,
            entities_count: world.entity_count(),
            phases_executed: self.phases.len(),
            systems_executed,
            current_tick,
        }
    }

    /// Returns the number of compiled phases in this schedule.
    #[inline(always)]
    pub fn phase_count(&self) -> usize {
        self.phases.len()
    }
}

/// Builder for constructing and validating a `Schedule`.
#[derive(Default)]
pub struct ScheduleBuilder {
    configs: Vec<PhaseConfig>,
    systems: HashMap<PhaseId, Vec<Box<dyn System>>>,
}

impl ScheduleBuilder {
    /// Creates a new empty `ScheduleBuilder`.
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
            systems: HashMap::new(),
        }
    }

    /// Registers a phase without explicit dependencies.
    pub fn add_phase(mut self, id: PhaseId) -> Self {
        if !self.configs.iter().any(|c| c.id == id) {
            self.configs.push(PhaseConfig::new(id));
        }
        self
    }

    /// Registers a phase with explicit `before` and `after` dependency constraints.
    pub fn add_phase_with_dependencies(
        mut self,
        id: PhaseId,
        before: Vec<PhaseId>,
        after: Vec<PhaseId>,
    ) -> Self {
        if let Some(existing) = self.configs.iter_mut().find(|c| c.id == id) {
            existing.before.extend(before);
            existing.after.extend(after);
        } else {
            self.configs.push(PhaseConfig { id, before, after });
        }
        self
    }

    /// Adds an executable system to the specified phase.
    pub fn add_system<S: System>(mut self, phase: PhaseId, system: S) -> Self {
        self = self.add_phase(phase.clone());
        self.systems.entry(phase).or_default().push(Box::new(system));
        self
    }

    /// Adds a closure function system to the specified phase.
    pub fn add_function_system<S: Into<String>, F>(
        self,
        phase: PhaseId,
        name: S,
        func: F,
    ) -> Self
    where
        F: FnMut(&mut World) + 'static + Send + Sync,
    {
        self.add_system(phase, FunctionSystem::new(name, func))
    }

    /// Compiles and validates the DAG schedule, sorting phases in topological order.
    ///
    /// Returns `Err(EcsError)` if a cycle or missing dependency is detected.
    pub fn build(self) -> Result<Schedule, EcsError> {
        let sorted_ids = sort_phases(&self.configs)?;
        let mut systems_map = self.systems;

        let phases = sorted_ids
            .into_iter()
            .map(|id| {
                let systems = systems_map.remove(&id).unwrap_or_default();
                CompiledPhase { id, systems }
            })
            .collect();

        Ok(Schedule { phases })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq, Clone)]
    struct Counter(u32);

    #[test]
    fn schedule_execution_order() {
        let mut world = World::new();
        world.insert_singleton(Counter(0));

        let mut schedule = Schedule::builder()
            .add_phase(PhaseId::PreUpdate)
            .add_phase_with_dependencies(
                PhaseId::Update,
                vec![PhaseId::RenderSubmit],
                vec![PhaseId::PreUpdate],
            )
            .add_phase(PhaseId::RenderSubmit)
            .add_function_system(PhaseId::PreUpdate, "sys_pre", |w| {
                if let Some(c) = w.singleton_mut::<Counter>() {
                    c.0 += 10;
                }
            })
            .add_function_system(PhaseId::Update, "sys_update", |w| {
                if let Some(c) = w.singleton_mut::<Counter>() {
                    c.0 *= 2;
                }
            })
            .add_function_system(PhaseId::RenderSubmit, "sys_submit", |w| {
                if let Some(c) = w.singleton_mut::<Counter>() {
                    c.0 += 1;
                }
            })
            .build()
            .unwrap();

        // Execution order: (0 + 10) * 2 + 1 = 21
        let report = schedule.run(&mut world);
        assert_eq!(report.phases_executed, 3);
        assert_eq!(report.systems_executed, 3);
        assert_eq!(world.singleton::<Counter>(), Some(&Counter(21)));
    }
}
