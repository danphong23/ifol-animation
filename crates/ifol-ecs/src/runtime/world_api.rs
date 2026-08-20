use super::ecs_runtime::EcsRuntime;
use crate::entity::EntityId;
use crate::error::{EcsError, SystemError};
use crate::query::{Query, QueryMut, WorldQuery, WorldQueryMut};
use crate::storage::Component;
use crate::world::World;

impl EcsRuntime {
    /// Returns the runtime's ECS world.
    #[inline(always)]
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Returns mutable access to the runtime's ECS world.
    #[inline(always)]
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    #[inline(always)]
    pub fn spawn(&mut self) -> EntityId {
        self.world.spawn()
    }

    #[inline(always)]
    pub fn despawn(&mut self, entity: EntityId) -> Result<(), EcsError> {
        self.world.despawn(entity)
    }

    #[inline(always)]
    pub fn insert<T: Component>(
        &mut self,
        entity: EntityId,
        component: T,
    ) -> Result<Option<T>, EcsError> {
        self.world.insert(entity, component)
    }

    #[inline(always)]
    pub fn get<T: Component>(&self, entity: EntityId) -> Option<&T> {
        self.world.get::<T>(entity)
    }

    #[inline(always)]
    pub fn get_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        self.world.get_mut::<T>(entity)
    }

    #[inline(always)]
    pub fn remove<T: Component>(&mut self, entity: EntityId) -> Option<T> {
        self.world.remove::<T>(entity)
    }

    #[inline(always)]
    pub fn insert_world_component<T: Component>(&mut self, component: T) -> Option<T> {
        self.world.insert_world_component(component)
    }

    #[inline(always)]
    pub fn get_world_component<T: Component>(&self) -> Option<&T> {
        self.world.get_world_component::<T>()
    }

    #[inline(always)]
    pub fn get_world_component_mut<T: Component>(&mut self) -> Option<&mut T> {
        self.world.get_world_component_mut::<T>()
    }

    #[inline(always)]
    pub fn has_world_component<T: Component>(&self) -> bool {
        self.world.has_world_component::<T>()
    }

    #[inline(always)]
    pub fn remove_world_component<T: Component>(&mut self) -> Option<T> {
        self.world.remove_world_component::<T>()
    }

    #[inline(always)]
    pub fn query<Q: WorldQuery>(&self) -> Query<'_, Q> {
        self.world.query::<Q>()
    }

    /// Creates a mutable query over the runtime world after validating aliasing.
    #[inline]
    pub fn query_mut<Q: WorldQueryMut>(&mut self) -> Result<QueryMut<'_, Q>, SystemError> {
        self.world.query_mut::<Q>()
    }

    /// Returns query-plan cache statistics `(hits, misses)`.
    #[inline]
    pub fn query_plan_cache_stats(&self) -> (usize, usize) {
        self.world.query_plan_cache_stats()
    }

    /// Invalidates the compiled schedule after registration changes.
    pub fn reconfigure(&mut self) {
        self.compiled_schedule = None;
    }

    /// Clears world data while preserving registrations.
    pub fn clear(&mut self) {
        self.world.clear();
    }

    /// Shuts down the runtime, clearing all data, registrations, and caches.
    pub fn shutdown(&mut self) {
        self.world = World::new();
        self.phase_registry = crate::registry::PhaseRegistry::new();
        self.system_registry = crate::registry::SystemRegistry::new();
        self.compiled_schedule = None;
        self.commands = crate::system::Commands::new();
        self.execution_counter = 0;
        self.execution_policy = crate::report::ExecutionPolicy::default();
    }
}
