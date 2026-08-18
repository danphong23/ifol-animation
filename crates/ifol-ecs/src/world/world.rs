use crate::entity::{EntityId, EntityManager};
use crate::error::{EcsError, SystemError};
use crate::query::{Query, QueryMut, QueryPlanCache, QueryPlanKey, WorldQuery, WorldQueryMut};
use crate::registry::{ComponentId, ComponentRegistry};
use crate::storage::{AnyStorage, Component, SparseSet};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;

/// The central container owning entities, component storages, and revision state.
pub struct World {
    entities: EntityManager,
    component_registry: ComponentRegistry,
    storages: HashMap<TypeId, Box<dyn AnyStorage>>,
    structural_version: u64,
    current_tick: u64,
    query_plan_cache: RefCell<QueryPlanCache>,
}

impl World {
    /// Creates a new empty `World` with `current_tick = 1` and pre-spawned `WORLD_ENTITY`.
    pub fn new() -> Self {
        Self {
            entities: EntityManager::new(),
            component_registry: ComponentRegistry::new(),
            storages: HashMap::new(),
            structural_version: 0,
            current_tick: 1,
            query_plan_cache: RefCell::new(QueryPlanCache::new()),
        }
    }

    /// Spawns a new entity in the world and increments `structural_version`.
    pub fn spawn(&mut self) -> EntityId {
        self.structural_version = self.structural_version.wrapping_add(1);
        self.query_plan_cache.borrow_mut().clear();
        self.entities.spawn()
    }

    /// Despawns an entity and removes all its attached components across all storages.
    ///
    /// Returns `Err(EcsError::EntityNotFound)` if the entity is invalid, dead, or the root WORLD entity.
    pub fn despawn(&mut self, entity: EntityId) -> Result<(), EcsError> {
        self.entities.despawn(entity)?;
        for storage in self.storages.values_mut() {
            storage.remove_entity(entity);
        }
        self.structural_version = self.structural_version.wrapping_add(1);
        self.query_plan_cache.borrow_mut().clear();
        Ok(())
    }

    /// Returns `true` if the entity is currently alive.
    #[inline(always)]
    pub fn is_alive(&self, entity: EntityId) -> bool {
        self.entities.is_alive(entity)
    }

    /// Returns the total count of alive entities (including `WORLD_ENTITY`).
    #[inline(always)]
    pub fn entity_count(&self) -> usize {
        self.entities.alive_count()
    }

    /// Returns all currently alive entity IDs, including `WORLD_ENTITY`.
    pub fn alive_entities(&self) -> Vec<EntityId> {
        self.entities.iter_alive().collect()
    }

    /// Returns the current monotonic structural version.
    #[inline(always)]
    pub fn structural_version(&self) -> u64 {
        self.structural_version
    }

    /// Returns query-plan cache statistics `(hits, misses)`.
    pub fn query_plan_cache_stats(&self) -> (usize, usize) {
        self.query_plan_cache.borrow().stats()
    }

    /// Returns the current execution tick counter.
    #[inline(always)]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Increments the world tick counter (called at the end of each `run_once` pass).
    #[inline(always)]
    pub fn increment_tick(&mut self) {
        self.current_tick = self.current_tick.wrapping_add(1);
    }

    /// Returns a reference to the component registry.
    #[inline(always)]
    pub fn component_registry(&self) -> &ComponentRegistry {
        &self.component_registry
    }

    /// Returns the registered component ID for `T`, if present.
    #[inline]
    pub fn component_id<T: Component>(&self) -> Option<ComponentId> {
        self.component_registry.get_id::<T>()
    }

    /// Returns a mutable reference to the component registry.
    #[inline(always)]
    pub fn component_registry_mut(&mut self) -> &mut ComponentRegistry {
        &mut self.component_registry
    }

    /// Inserts a component into the given entity.
    ///
    /// Automatically registers the component type if not already present.
    /// Increments `structural_version` if the component was not already attached.
    pub fn insert<T: Component>(
        &mut self,
        entity: EntityId,
        component: T,
    ) -> Result<Option<T>, EcsError> {
        self.entities.validate(entity)?;

        // Ensure component is registered in registry
        if self.component_registry.get_id::<T>().is_none() {
            let _ = self.component_registry.register::<T>();
        }

        let current_tick = self.current_tick;
        let storage = self.get_or_insert_storage::<T>();
        let old = storage.insert(entity, component, current_tick);
        if old.is_none() {
            self.structural_version = self.structural_version.wrapping_add(1);
            self.query_plan_cache.borrow_mut().clear();
        }
        Ok(old)
    }

    /// Removes a component from the given entity and increments `structural_version`.
    pub fn remove<T: Component>(&mut self, entity: EntityId) -> Option<T> {
        let storage = self.storages.get_mut(&TypeId::of::<T>())?;
        let sparse_set = storage.as_any_mut().downcast_mut::<SparseSet<T>>()?;
        let removed = sparse_set.remove(entity);
        if removed.is_some() {
            self.structural_version = self.structural_version.wrapping_add(1);
            self.query_plan_cache.borrow_mut().clear();
        }
        removed
    }

    /// Retrieves an immutable reference to a component on the given entity.
    pub fn get<T: Component>(&self, entity: EntityId) -> Option<&T> {
        if !self.entities.is_alive(entity) {
            return None;
        }
        let storage = self.storages.get(&TypeId::of::<T>())?;
        let sparse_set = storage.as_any().downcast_ref::<SparseSet<T>>()?;
        sparse_set.get(entity)
    }

    /// Retrieves a mutable reference to a component on the given entity and updates its change tick.
    pub fn get_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        if !self.entities.is_alive(entity) {
            return None;
        }
        let current_tick = self.current_tick;
        let storage = self.storages.get_mut(&TypeId::of::<T>())?;
        let sparse_set = storage.as_any_mut().downcast_mut::<SparseSet<T>>()?;
        sparse_set.get_mut(entity, current_tick)
    }

    /// Retrieves the last changed tick of a component on the given entity.
    pub fn get_tick<T: Component>(&self, entity: EntityId) -> Option<u64> {
        if !self.entities.is_alive(entity) {
            return None;
        }
        let storage = self.storages.get(&TypeId::of::<T>())?;
        let sparse_set = storage.as_any().downcast_ref::<SparseSet<T>>()?;
        sparse_set.get_tick(entity)
    }

    /// Returns `true` if the entity has a component of type `T`.
    pub fn has_component<T: Component>(&self, entity: EntityId) -> bool {
        if !self.entities.is_alive(entity) {
            return false;
        }
        if let Some(storage) = self.storages.get(&TypeId::of::<T>())
            && let Some(sparse_set) = storage.as_any().downcast_ref::<SparseSet<T>>()
        {
            return sparse_set.contains(entity);
        }
        false
    }

    /// Returns `true` if the root `WORLD_ENTITY` has a component matching the `ComponentId`.
    pub fn has_world_component_by_id(&self, id: ComponentId) -> bool {
        if let Some(desc) = self.component_registry.descriptor(id)
            && let Some(storage) = self.storages.get(&desc.type_id)
        {
            return storage.contains_entity(EntityId::WORLD);
        }
        false
    }

    /// Returns an immutable reference to the underlying `SparseSet<T>` storage.
    pub fn storage<T: Component>(&self) -> Option<&SparseSet<T>> {
        let storage = self.storages.get(&TypeId::of::<T>())?;
        storage.as_any().downcast_ref::<SparseSet<T>>()
    }

    pub(crate) fn cached_query_candidates(
        &self,
        key: QueryPlanKey,
        build: impl FnOnce() -> Vec<EntityId>,
    ) -> Vec<EntityId> {
        let mut cache = self.query_plan_cache.borrow_mut();
        if let Some(entities) = cache.get(&key) {
            entities.to_vec()
        } else {
            let entities = build();
            cache.insert(key, entities.clone());
            entities
        }
    }

    /// Queries entities matching the specified `WorldQuery` pattern.
    #[inline]
    pub fn query<Q: WorldQuery>(&self) -> Query<'_, Q> {
        Query::new(self)
    }

    /// Creates a mutable query after validating that its signature has no aliasing.
    pub fn query_mut<Q: WorldQueryMut>(&mut self) -> Result<QueryMut<'_, Q>, SystemError> {
        Q::access().validate_mutable().map_err(SystemError::new)?;
        Ok(QueryMut::new(self))
    }

    /// Clears all entities and component storages while preserving `WORLD_ENTITY`.
    pub fn clear(&mut self) {
        self.entities = EntityManager::new();
        self.storages.clear();
        self.structural_version = self.structural_version.wrapping_add(1);
        self.query_plan_cache.borrow_mut().clear();
    }

    fn get_or_insert_storage<T: Component>(&mut self) -> &mut SparseSet<T> {
        let type_id = TypeId::of::<T>();
        self.storages
            .entry(type_id)
            .or_insert_with(|| Box::new(SparseSet::<T>::new()))
            .as_any_mut()
            .downcast_mut::<SparseSet<T>>()
            .expect("storage type downcast invariant")
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    struct Pos {
        x: i32,
        y: i32,
    }

    #[test]
    fn world_spawn_insert_and_despawn() {
        let mut world = World::new();
        assert_eq!(world.entity_count(), 1); // WORLD entity
        assert_eq!(world.structural_version(), 0);

        let e1 = world.spawn();
        assert_eq!(world.structural_version(), 1);

        world.insert(e1, Pos { x: 10, y: 20 }).unwrap();
        assert_eq!(world.structural_version(), 2);
        assert_eq!(world.get::<Pos>(e1), Some(&Pos { x: 10, y: 20 }));

        // Replacing existing component should not increment structural version
        world.insert(e1, Pos { x: 30, y: 40 }).unwrap();
        assert_eq!(world.structural_version(), 2);
        assert_eq!(world.get::<Pos>(e1), Some(&Pos { x: 30, y: 40 }));

        // Despawn entity
        assert!(world.despawn(e1).is_ok());
        assert_eq!(world.structural_version(), 3);
        assert_eq!(world.get::<Pos>(e1), None);
    }
}
