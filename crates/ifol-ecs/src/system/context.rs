use crate::entity::EntityId;
use crate::error::SystemError;
use crate::query::{Query, QueryAccess, WorldQuery};
use crate::registry::{ComponentId, SystemId};
use crate::storage::Component;
use crate::system::AccessDescriptor;
use crate::system::Commands;
use crate::world::World;
use std::any::TypeId;

/// Execution context provided to a system during its execution pass.
///
/// Encapsulates safe tracked access to World components, World singletons, and deferred commands.
pub struct SystemContext<'a> {
    world: &'a mut World,
    commands: &'a mut Commands,
    system_id: SystemId,
    system_name: String,
    access: AccessDescriptor,
}

impl<'a> SystemContext<'a> {
    /// Creates a new `SystemContext`.
    pub fn new(
        world: &'a mut World,
        commands: &'a mut Commands,
        system_id: SystemId,
        system_name: String,
        access: AccessDescriptor,
    ) -> Self {
        Self {
            world,
            commands,
            system_id,
            system_name,
            access,
        }
    }

    fn component_id<T: Component>(&self) -> Result<ComponentId, SystemError> {
        self.world.component_id::<T>().ok_or_else(|| {
            SystemError::new(format!(
                "component '{}' is not registered",
                std::any::type_name::<T>()
            ))
        })
    }

    fn check_type_access(
        &self,
        type_id: TypeId,
        mode: &str,
        allowed: impl FnOnce(&AccessDescriptor, ComponentId) -> bool,
    ) -> Result<(), SystemError> {
        let component_id = self
            .world
            .component_registry()
            .get_id_by_type_id(type_id)
            .ok_or_else(|| {
                SystemError::new(format!("component type {type_id:?} is not registered"))
            })?;
        if allowed(&self.access, component_id) {
            Ok(())
        } else {
            Err(SystemError::access_denied(format!("{type_id:?}"), mode))
        }
    }

    fn check_query_access(&self, access: QueryAccess) -> Result<(), SystemError> {
        for type_id in access.reads {
            self.check_type_access(type_id, "read", AccessDescriptor::allows_read)?;
        }
        for type_id in access.writes {
            self.check_type_access(type_id, "write", AccessDescriptor::allows_write)?;
        }
        Ok(())
    }

    /// Queries entities matching the specified `WorldQuery` pattern.
    #[inline]
    pub fn query<Q: WorldQuery>(&self) -> Result<Query<'_, Q>, SystemError> {
        self.check_query_access(Q::access())?;
        Ok(self.world.query::<Q>())
    }

    /// Retrieves an immutable reference to a component on the given entity.
    #[inline]
    pub fn get<T: Component>(&self, entity: EntityId) -> Result<Option<&T>, SystemError> {
        let id = self.component_id::<T>()?;
        if !self.access.allows_read(id) {
            return Err(SystemError::access_denied(
                std::any::type_name::<T>(),
                "read",
            ));
        }
        Ok(self.world.get::<T>(entity))
    }

    /// Retrieves a mutable reference to a component on the given entity.
    #[inline]
    pub fn get_mut<T: Component>(
        &mut self,
        entity: EntityId,
    ) -> Result<Option<&mut T>, SystemError> {
        let id = self.component_id::<T>()?;
        if !self.access.allows_write(id) {
            return Err(SystemError::access_denied(
                std::any::type_name::<T>(),
                "write",
            ));
        }
        Ok(self.world.get_mut::<T>(entity))
    }

    /// Retrieves an immutable reference to a world singleton component on `WORLD_ENTITY`.
    #[inline]
    pub fn world_ref<T: Component>(&self) -> Result<Option<&T>, SystemError> {
        let id = self.component_id::<T>()?;
        if !self.access.allows_read(id) {
            return Err(SystemError::access_denied(
                std::any::type_name::<T>(),
                "read",
            ));
        }
        Ok(self.world.get_world_component::<T>())
    }

    /// Retrieves a mutable reference to a world singleton component on `WORLD_ENTITY`.
    #[inline]
    pub fn world_mut<T: Component>(&mut self) -> Result<Option<&mut T>, SystemError> {
        let id = self.component_id::<T>()?;
        if !self.access.allows_write(id) {
            return Err(SystemError::access_denied(
                std::any::type_name::<T>(),
                "write",
            ));
        }
        Ok(self.world.get_world_component_mut::<T>())
    }

    /// Returns a mutable reference to the deferred `Commands` buffer.
    #[inline(always)]
    pub fn commands(&mut self) -> &mut Commands {
        self.commands
    }

    /// Returns the system ID of the currently executing system.
    #[inline(always)]
    pub fn system_id(&self) -> SystemId {
        self.system_id
    }

    /// Returns the diagnostic name of the currently executing system.
    #[inline(always)]
    pub fn system_name(&self) -> &str {
        &self.system_name
    }
}
