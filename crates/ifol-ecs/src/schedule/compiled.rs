use crate::error::EcsError;
use crate::registry::{PhaseId, PhaseRegistry, SystemId, SystemRegistry};
use crate::report::{ExecutionPolicy, RunReport, SkippedSystem};
use crate::schedule::graph::PhaseGraph;
use crate::system::{Commands, SystemContext};
use crate::world::World;
use std::collections::HashSet;
use std::time::Instant;

/// A compiled phase containing ordered system IDs.
#[derive(Debug, Clone)]
pub struct CompiledPhase {
    pub id: PhaseId,
    pub systems: Vec<SystemId>,
}

/// An immutable, compiled execution schedule owned by `EcsRuntime`.
pub struct CompiledSchedule {
    phases: Vec<CompiledPhase>,
    graph_revision: u64,
}

impl CompiledSchedule {
    /// Compiles the phase graph from `PhaseRegistry` into an ordered `CompiledSchedule`.
    pub fn compile(registry: &PhaseRegistry, systems: &SystemRegistry) -> Result<Self, EcsError> {
        let order = PhaseGraph::compile_order(registry)?;
        let mut phases = Vec::with_capacity(order.len());

        for phase_id in order {
            let node = registry
                .phases()
                .get(&phase_id)
                .ok_or_else(|| EcsError::PhaseNotFound(phase_id.to_string()))?;
            let mut seen = HashSet::new();
            for system_id in node.system_bindings() {
                let registration = systems
                    .get(*system_id)
                    .ok_or_else(|| EcsError::SystemNotFound(format!("binding {system_id:?}")))?;
                if !seen.insert(*system_id) {
                    return Err(EcsError::DuplicateSystemBinding {
                        phase: phase_id.to_string(),
                        system: registration.name.clone(),
                    });
                }
            }
            phases.push(CompiledPhase {
                id: phase_id,
                systems: node.system_bindings().to_vec(),
            });
        }

        Ok(Self {
            phases,
            graph_revision: registry.revision(),
        })
    }

    /// Executes one execution pass over the world.
    pub fn run_pass(
        &mut self,
        world: &mut World,
        systems: &mut SystemRegistry,
        commands: &mut Commands,
        execution_revision: u64,
        execution_policy: ExecutionPolicy,
    ) -> Result<RunReport, EcsError> {
        let start_time = Instant::now();

        let mut report = RunReport {
            execution_revision,
            compiled_graph_revision: self.graph_revision,
            phases_visited: Vec::with_capacity(self.phases.len()),
            systems_executed: Vec::new(),
            systems_skipped: Vec::new(),
            commands_processed: 0,
            system_errors: Vec::new(),
            structural_version: world.structural_version(),
            entities_count: world.entity_count(),
            duration_us: 0,
        };

        for phase in &self.phases {
            report.phases_visited.push(phase.id.to_string());

            for &sys_id in &phase.systems {
                let sys_reg = systems
                    .get_mut(sys_id)
                    .ok_or_else(|| EcsError::SystemNotFound(format!("binding {sys_id:?}")))?;

                let sys_name = sys_reg.name.clone();

                // Evaluate run conditions
                let mut condition_failed = None;
                for condition in &sys_reg.conditions {
                    if let Err(reason) = condition.evaluate(world) {
                        condition_failed = Some(reason);
                        break;
                    }
                }

                if let Some(reason) = condition_failed {
                    report.systems_skipped.push(SkippedSystem {
                        system: sys_name,
                        reason,
                    });
                    continue;
                }

                // Execute system with a checked SystemContext.
                let mut ctx = SystemContext::new(
                    world,
                    commands,
                    sys_id,
                    sys_name.clone(),
                    sys_reg.access.clone(),
                );

                match sys_reg.system.run(&mut ctx) {
                    Ok(()) => {
                        report.systems_executed.push(sys_name);
                    }
                    Err(err) => {
                        commands.clear();
                        match execution_policy {
                            ExecutionPolicy::CollectErrors => {
                                report.system_errors.push((sys_name, err));
                            }
                            ExecutionPolicy::StopPhaseOnError => {
                                report.system_errors.push((sys_name, err));
                                break;
                            }
                            ExecutionPolicy::FailFast => {
                                return Err(EcsError::SystemExecutionFailed {
                                    system: sys_name,
                                    error: err,
                                });
                            }
                        }
                    }
                }

                // Flush commands at intra-phase safe point
                let flushed = commands.apply(world)?;
                report.commands_processed += flushed;
            }

            // Flush commands at phase boundary safe point
            let flushed = commands.apply(world)?;
            report.commands_processed += flushed;
        }

        // Advance world tick counter after completing the execution pass
        world.increment_tick();

        report.structural_version = world.structural_version();
        report.entities_count = world.entity_count();
        report.duration_us = start_time.elapsed().as_micros() as u64;

        Ok(report)
    }

    /// Returns the number of compiled phases in the schedule.
    #[inline(always)]
    pub fn phase_count(&self) -> usize {
        self.phases.len()
    }

    /// Returns the graph revision used when compiling this schedule.
    #[inline(always)]
    pub fn graph_revision(&self) -> u64 {
        self.graph_revision
    }
}
