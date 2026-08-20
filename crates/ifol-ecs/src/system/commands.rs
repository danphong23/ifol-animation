use crate::error::SystemError;
use crate::registry::ComponentRegistry;
use crate::storage::Component;
use crate::system::access::AccessDescriptor;

pub use crate::system::command_buffer::{CommandEntity, Commands, SpawnTicket};

/// Access-checked command facade exposed to a running system.
pub struct SystemCommands<'a> {
    commands: &'a mut Commands,
    component_registry: &'a ComponentRegistry,
    access: &'a AccessDescriptor,
}

impl<'a> SystemCommands<'a> {
    pub(crate) fn new(
        commands: &'a mut Commands,
        component_registry: &'a ComponentRegistry,
        access: &'a AccessDescriptor,
    ) -> Self {
        Self {
            commands,
            component_registry,
            access,
        }
    }

    /// Queues a spawn and returns its same-buffer ticket.
    #[inline]
    pub fn spawn(&mut self) -> SpawnTicket {
        self.commands.spawn()
    }

    /// Queues a despawn after checking the structural access contract.
    #[inline]
    pub fn despawn(&mut self, target: impl Into<CommandEntity>) -> Result<(), SystemError> {
        if !self.access.allows_structural() {
            return Err(SystemError::structural_access_denied());
        }
        self.commands.despawn(target);
        Ok(())
    }

    fn check_write<T: Component>(&self) -> Result<(), SystemError> {
        let id = self.component_registry.get_id::<T>().ok_or_else(|| {
            SystemError::new(format!(
                "component '{}' is not registered",
                std::any::type_name::<T>()
            ))
        })?;
        if self.access.allows_write(id) {
            Ok(())
        } else {
            Err(SystemError::access_denied(
                std::any::type_name::<T>(),
                "write",
            ))
        }
    }

    /// Queues a component insertion after checking the system write contract.
    pub fn insert<T: Component>(
        &mut self,
        target: impl Into<CommandEntity>,
        component: T,
    ) -> Result<(), SystemError> {
        self.check_write::<T>()?;
        self.commands.insert(target, component);
        Ok(())
    }

    /// Queues a component removal after checking the system write contract.
    pub fn remove<T: Component>(
        &mut self,
        target: impl Into<CommandEntity>,
    ) -> Result<(), SystemError> {
        self.check_write::<T>()?;
        self.commands.remove::<T>(target);
        Ok(())
    }
}
