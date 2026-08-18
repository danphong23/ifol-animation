use crate::storage::Component;
use crate::world::World;

impl World {
    /// Inserts a world singleton component onto the root `WORLD_ENTITY`.
    ///
    /// Returns the old component value if one was already present.
    pub fn insert_world_component<T: Component>(&mut self, component: T) -> Option<T> {
        self.component_registry_mut()
            .ensure_world_singleton::<T>()
            .expect("world singleton registration invariant");
        self.insert(crate::entity::EntityId::WORLD, component)
            .expect("WORLD_ENTITY is always alive")
    }

    /// Retrieves an immutable reference to a world singleton component on `WORLD_ENTITY`.
    pub fn get_world_component<T: Component>(&self) -> Option<&T> {
        self.get::<T>(crate::entity::EntityId::WORLD)
    }

    /// Retrieves a mutable reference to a world singleton component on `WORLD_ENTITY`.
    pub fn get_world_component_mut<T: Component>(&mut self) -> Option<&mut T> {
        self.get_mut::<T>(crate::entity::EntityId::WORLD)
    }

    /// Checks if a world singleton component of type `T` exists on `WORLD_ENTITY`.
    pub fn has_world_component<T: Component>(&self) -> bool {
        self.has_component::<T>(crate::entity::EntityId::WORLD)
    }

    /// Removes a world singleton component of type `T` from `WORLD_ENTITY`.
    pub fn remove_world_component<T: Component>(&mut self) -> Option<T> {
        self.remove::<T>(crate::entity::EntityId::WORLD)
    }
}
