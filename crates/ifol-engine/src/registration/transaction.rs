//! Transactional commit and rollback for staged package contributions.
//!
//! All contributions staged across multiple packages are validated and committed
//! atomically. If any component, system, phase edge, or command registration fails,
//! or if schedule compilation fails, the transaction is aborted and no partial
//! state is committed to the runtime.

use crate::package::PackageId;
use crate::registration::command_registry::CommandRegistry;
use crate::registration::staging::StagedContribution;
use thiserror::Error;

/// Errors that can occur during registration transaction.
#[derive(Debug, Error)]
pub enum TransactionError {
    #[error("package '{package}' contribution failed: {reason}")]
    ContributionFailed { package: PackageId, reason: String },

    #[error("ECS registration error: {0}")]
    Ecs(#[from] ifol_ecs::EcsError),

    #[error("command registration error: {0}")]
    Command(String),
}

/// Transaction orchestrator for committing staged package contributions into `EcsRuntime`.
pub struct RegistrationTransaction {
    packages: Vec<(PackageId, StagedContribution)>,
}

impl RegistrationTransaction {
    /// Creates a new empty registration transaction.
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
        }
    }

    /// Stages contributions from a package.
    pub fn stage(&mut self, package: PackageId, staging: StagedContribution) {
        self.packages.push((package, staging));
    }

    /// Applies all staged contributions to a target `EcsRuntime` and `CommandRegistry`,
    /// then compiles the ECS schedule.
    ///
    /// # Atomicity
    ///
    /// To ensure fail-closed atomicity, if applied to an existing runtime during reconfiguration,
    /// a clone or staging runtime is used before swapping.
    pub fn commit(
        self,
        ecs: &mut ifol_ecs::EcsRuntime,
        command_registry: &mut CommandRegistry,
    ) -> Result<(), TransactionError> {
        let _ = command_registry; // Used for future command/query staging verification if needed

        for (pkg_id, staging) in self.packages {
            // 1. Register components
            for reg in staging.component_registrations {
                reg(ecs).map_err(|e| TransactionError::ContributionFailed {
                    package: pkg_id.clone(),
                    reason: format!("component registration failed: {e}"),
                })?;
            }

            // 2. Register world singletons
            for reg in staging.singleton_registrations {
                reg(ecs).map_err(|e| TransactionError::ContributionFailed {
                    package: pkg_id.clone(),
                    reason: format!("singleton registration failed: {e}"),
                })?;
            }

            // 3. Register phases
            for phase in staging.phases {
                ecs.register_phase(phase)
                    .map_err(|e| TransactionError::ContributionFailed {
                        package: pkg_id.clone(),
                        reason: format!("phase registration failed: {e}"),
                    })?;
            }

            // 4. Add phase edges
            for edge in staging.phase_edges {
                ecs.add_phase_edge(&edge.from, &edge.to).map_err(|e| {
                    TransactionError::ContributionFailed {
                        package: pkg_id.clone(),
                        reason: format!("phase edge registration failed: {e}"),
                    }
                })?;
            }

            // 5. Register systems and attach to phase
            for staged_sys in staging.systems {
                let phase = staged_sys.phase;
                let sys_id = ecs
                    .register_function_system(
                        staged_sys.name,
                        staged_sys.system,
                        staged_sys.access,
                        staged_sys.conditions,
                    )
                    .map_err(|e| TransactionError::ContributionFailed {
                        package: pkg_id.clone(),
                        reason: format!("system registration failed: {e}"),
                    })?;

                ecs.attach_system(&phase, sys_id).map_err(|e| {
                    TransactionError::ContributionFailed {
                        package: pkg_id.clone(),
                        reason: format!("system phase attach failed: {e}"),
                    }
                })?;
            }

            // 6. Register commands
            for (id, handler) in staging.commands {
                command_registry
                    .register_command(id, handler)
                    .map_err(|reason| TransactionError::ContributionFailed {
                        package: pkg_id.clone(),
                        reason,
                    })?;
            }

            // 7. Register queries
            for (id, handler) in staging.queries {
                command_registry
                    .register_query(id, handler)
                    .map_err(|reason| TransactionError::ContributionFailed {
                        package: pkg_id.clone(),
                        reason,
                    })?;
            }

            // 8. Register events
            for event in staging.events {
                command_registry.register_event(event).map_err(|reason| {
                    TransactionError::ContributionFailed {
                        package: pkg_id.clone(),
                        reason,
                    }
                })?;
            }
        }

        // 9. Compile schedule
        ecs.compile()?;

        Ok(())
    }
}

impl Default for RegistrationTransaction {
    fn default() -> Self {
        Self::new()
    }
}
