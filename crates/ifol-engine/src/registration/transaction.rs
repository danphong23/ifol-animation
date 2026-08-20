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

    #[error("schema registration error: {0}")]
    Schema(#[from] crate::scene::CodecError),

    #[error("migration registration error: {0}")]
    Migration(#[from] crate::scene::MigrationError),
}

/// Transaction orchestrator for preparing staged package contributions.
///
/// The commit operation consumes its ECS and command-registry candidates and
/// returns them only after every contribution and schedule compilation step
/// succeeds. A failed commit therefore cannot partially mutate a live runtime.
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

    /// Stages package contributions collected via a `RegistrationContext`.
    pub fn stage_package<F>(&mut self, package: PackageId, f: F)
    where
        F: FnOnce(&mut crate::registration::RegistrationContext),
    {
        let mut ctx = crate::registration::RegistrationContext::new(package.clone());
        f(&mut ctx);
        self.stage(package, ctx.into_staging());
    }

    /// Applies all staged contributions to staging candidates and compiles the
    /// ECS schedule.
    ///
    /// # Atomicity
    ///
    /// The candidates are consumed and returned only on success. Callers must
    /// pass a fresh staging runtime when replacing an existing live runtime.
    pub(crate) fn commit(
        self,
        mut ecs: ifol_ecs::EcsRuntime,
        mut command_registry: CommandRegistry,
        mut schemas: crate::scene::SchemaRegistry,
        mut migrations: crate::scene::MigrationRegistry,
        mut provider_manager: crate::provider::ProviderManager,
        mut namespaces: crate::namespace::NamespaceRegistry,
    ) -> Result<
        (
            ifol_ecs::EcsRuntime,
            CommandRegistry,
            crate::scene::SchemaRegistry,
            crate::scene::MigrationRegistry,
            crate::provider::ProviderManager,
            crate::namespace::NamespaceRegistry,
        ),
        TransactionError,
    > {
        for (pkg_id, staging) in self.packages {
            // 1. Register components
            for reg in staging.component_registrations {
                reg(&mut ecs).map_err(|e| TransactionError::ContributionFailed {
                    package: pkg_id.clone(),
                    reason: format!("component registration failed: {e}"),
                })?;
            }

            // 2. Register world singletons
            for reg in staging.singleton_registrations {
                reg(&mut ecs).map_err(|e| TransactionError::ContributionFailed {
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

            // 9. Register package-owned schemas and migration steps
            for (schema, codec) in staging.schemas {
                schemas.register(schema, codec).map_err(|error| {
                    TransactionError::ContributionFailed {
                        package: pkg_id.clone(),
                        reason: format!("schema registration failed: {error}"),
                    }
                })?;
            }

            for (schema, from, to, migration) in staging.migrations {
                migrations
                    .register_step(schema, from, to, migration)
                    .map_err(|error| TransactionError::ContributionFailed {
                        package: pkg_id.clone(),
                        reason: format!("migration registration failed: {error}"),
                    })?;
            }

            for provider in staging.providers {
                provider_manager.add(provider);
            }

            for namespace in staging.namespaces {
                namespaces
                    .claim(pkg_id.clone(), namespace)
                    .map_err(|error| TransactionError::ContributionFailed {
                        package: pkg_id.clone(),
                        reason: format!("namespace claim failed: {error}"),
                    })?;
            }
        }

        // 10. Compile schedule
        ecs.compile()?;

        Ok((
            ecs,
            command_registry,
            schemas,
            migrations,
            provider_manager,
            namespaces,
        ))
    }
}

impl Default for RegistrationTransaction {
    fn default() -> Self {
        Self::new()
    }
}
