use crate::entity::EntityId;
use crate::query::{Query, WorldQuery};
use crate::registry::SystemId;
use crate::storage::Component;
use crate::system::Commands;
use crate::world::World;

/// Execution context provided to a system during its execution pass.
///
/// Encapsulates safe tracked access to World components, World singletons, and deferred commands.
pub struct SystemContext<'a> {
    world: &'a mut World,
    commands: &'a mut Commands,
    system_id: SystemId,
    system_name: &'static str,
}

impl<'a> SystemContext<'a> {
    /// Creates a new `SystemContext`.
    pub fn new(
        world: &'a mut World,
        commands: &'a mut Commands,
        system_id: SystemId,
        system_name: &'static str,
    ) -> Self {
        Self {
            world,
            commands,
            system_id,
            system_name,
        }
    }

    /// Queries entities matching the specified `WorldQuery` pattern.
    #[inline]
    pub fn query<Q: WorldQuery>(&self) -> Query<'_, Q> {
        self.world.query::<Q>()
    }

    /// Retrieves an immutable reference to a component on the given entity.
    #[inline]
    pub fn get<T: Component>(&self, entity: EntityId) -> Option<&T> {
        self.world.get::<T>(entity)
    }

    /// Retrieves a mutable reference to a component on the given entity.
    #[inline]
    pub fn get_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        self.world.get_mut::<T>(entity)
    }

    /// Retrieves an immutable reference to a world singleton component on `WORLD_ENTITY`.
    #[inline]
    pub fn world_ref<T: Component>(&self) -> Option<&T> {
        self.world.get_world_component::<T>()
    }

    /// Retrieves a mutable reference to a world singleton component on `WORLD_ENTITY`.
    #[inline]
    pub fn world_mut<T: Component>(&mut self) -> Option<&mut T> {
        self.world.get_world_component_mut::<T>()
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
    pub fn system_name(&self) -> &'static str {
        self.system_name
    }
}
