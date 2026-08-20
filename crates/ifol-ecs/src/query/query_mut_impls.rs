use super::WorldQueryMut;
use crate::entity::EntityId;
use crate::query::QueryAccess;
use crate::query::filter::{With, Without};
use crate::storage::Component;
use crate::world::World;

unsafe impl<T: Component> WorldQueryMut for &'static mut T {
    type Item<'w> = &'w mut T;
    fn access() -> QueryAccess {
        QueryAccess {
            reads: Vec::new(),
            writes: vec![std::any::TypeId::of::<T>()],
        }
    }
    fn has_driver() -> bool {
        true
    }
    fn driver_entities(world: &World) -> Vec<EntityId> {
        world
            .component_entities::<T>()
            .map(|e| e.to_vec())
            .unwrap_or_default()
    }
    fn matches(world: &World, entity: EntityId) -> bool {
        world.has_component::<T>(entity)
    }
    unsafe fn fetch<'w>(world: &'w mut World, entity: EntityId) -> Option<Self::Item<'w>> {
        world.get_mut::<T>(entity)
    }
}

unsafe impl<T: Component> WorldQueryMut for &'static T {
    type Item<'w> = &'w T;
    fn access() -> QueryAccess {
        QueryAccess {
            reads: vec![std::any::TypeId::of::<T>()],
            writes: Vec::new(),
        }
    }
    fn has_driver() -> bool {
        true
    }
    fn driver_entities(world: &World) -> Vec<EntityId> {
        world
            .component_entities::<T>()
            .map(|e| e.to_vec())
            .unwrap_or_default()
    }
    fn matches(world: &World, entity: EntityId) -> bool {
        world.has_component::<T>(entity)
    }
    unsafe fn fetch<'w>(world: &'w mut World, entity: EntityId) -> Option<Self::Item<'w>> {
        world.get::<T>(entity)
    }
}

unsafe impl<T: Component> WorldQueryMut for Option<&'static T> {
    type Item<'w> = Option<&'w T>;
    fn access() -> QueryAccess {
        QueryAccess {
            reads: vec![std::any::TypeId::of::<T>()],
            writes: Vec::new(),
        }
    }
    fn has_driver() -> bool {
        false
    }
    fn driver_entities(_world: &World) -> Vec<EntityId> {
        Vec::new()
    }
    fn matches(_world: &World, _entity: EntityId) -> bool {
        true
    }
    unsafe fn fetch<'w>(world: &'w mut World, entity: EntityId) -> Option<Self::Item<'w>> {
        Some(world.get::<T>(entity))
    }
}

unsafe impl<T: Component> WorldQueryMut for Option<&'static mut T> {
    type Item<'w> = Option<&'w mut T>;
    fn access() -> QueryAccess {
        QueryAccess {
            reads: Vec::new(),
            writes: vec![std::any::TypeId::of::<T>()],
        }
    }
    fn has_driver() -> bool {
        false
    }
    fn driver_entities(_world: &World) -> Vec<EntityId> {
        Vec::new()
    }
    fn matches(_world: &World, _entity: EntityId) -> bool {
        true
    }
    unsafe fn fetch<'w>(world: &'w mut World, entity: EntityId) -> Option<Self::Item<'w>> {
        Some(world.get_mut::<T>(entity))
    }
}

unsafe impl<T: Component> WorldQueryMut for With<T> {
    type Item<'w> = ();
    fn access() -> QueryAccess {
        QueryAccess {
            reads: vec![std::any::TypeId::of::<T>()],
            writes: Vec::new(),
        }
    }
    fn has_driver() -> bool {
        true
    }
    fn driver_entities(world: &World) -> Vec<EntityId> {
        world
            .component_entities::<T>()
            .map(|e| e.to_vec())
            .unwrap_or_default()
    }
    fn matches(world: &World, entity: EntityId) -> bool {
        world.has_component::<T>(entity)
    }
    unsafe fn fetch<'w>(_world: &'w mut World, _entity: EntityId) -> Option<Self::Item<'w>> {
        Some(())
    }
}

unsafe impl<T: Component> WorldQueryMut for Without<T> {
    type Item<'w> = ();
    fn access() -> QueryAccess {
        QueryAccess {
            reads: vec![std::any::TypeId::of::<T>()],
            writes: Vec::new(),
        }
    }
    fn has_driver() -> bool {
        false
    }
    fn driver_entities(_world: &World) -> Vec<EntityId> {
        Vec::new()
    }
    fn matches(world: &World, entity: EntityId) -> bool {
        !world.has_component::<T>(entity)
    }
    unsafe fn fetch<'w>(_world: &'w mut World, _entity: EntityId) -> Option<Self::Item<'w>> {
        Some(())
    }
}

unsafe impl WorldQueryMut for () {
    type Item<'w> = ();
    fn access() -> QueryAccess {
        QueryAccess::default()
    }
    fn has_driver() -> bool {
        false
    }
    fn driver_entities(world: &World) -> Vec<EntityId> {
        world.alive_entities()
    }
    fn matches(_world: &World, _entity: EntityId) -> bool {
        true
    }
    unsafe fn fetch<'w>(_world: &'w mut World, _entity: EntityId) -> Option<Self::Item<'w>> {
        Some(())
    }
}
