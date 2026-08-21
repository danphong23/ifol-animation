use super::EngineRuntime;
use crate::error::EngineError;
use crate::state::EngineState;

impl EngineRuntime {
    /// Replaces active packages and schedule through an atomic stage-and-swap transaction.
    pub fn reconfigure(
        &mut self,
        request: crate::reconfiguration::ReconfigurationRequest,
    ) -> Result<crate::reconfiguration::ReconfigurationReport, EngineError> {
        if self.state != EngineState::Ready {
            return Err(EngineError::InvalidState {
                expected: EngineState::Ready.label(),
                actual: self.state.label(),
            });
        }
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

        let staging_ecs = ifol_ecs::EcsRuntime::new();
        let (
            mut staging_ecs,
            staging_cmd_reg,
            staging_schemas,
            staging_migrations,
            mut provider_manager,
            staging_namespaces,
        ) = transaction.commit(
            staging_ecs,
            command_registry,
            schemas,
            migrations,
            provider_manager,
            namespaces,
        )?;
        provider_manager.init_all(&mut staging_ecs)?;

        if let Err(error) = self.provider_manager.teardown_all(&mut self.ecs) {
            let _ = provider_manager.teardown_all(&mut staging_ecs);
            self.state = EngineState::Faulted;
            return Err(EngineError::Provider(error));
        }

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
}
