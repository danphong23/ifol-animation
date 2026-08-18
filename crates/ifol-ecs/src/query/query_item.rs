use crate::entity::EntityId;
use crate::query::filter::{With, Without};
use crate::storage::Component;
use crate::world::World;

/// Trait implemented by types that can be fetched via an ECS query.
pub trait WorldQuery: 'static {
    /// The reference type yielded when querying the world for a specific entity.
    type Item<'w>;

    /// Returns candidate entity IDs from the most restrictive storage driver.
    fn driver_entities(world: &World) -> Vec<EntityId>;

    /// Returns `true` if the entity satisfies all query constraints.
    fn matches(world: &World, entity: EntityId) -> bool;

    /// Fetches the item for the matching entity.
    fn fetch<'w>(world: &'w World, entity: EntityId) -> Option<Self::Item<'w>>;
}

// Implement WorldQuery for &'static T
impl<T: Component> WorldQuery for &'static T {
    type Item<'w> = &'w T;

    fn driver_entities(world: &World) -> Vec<EntityId> {
        world
            .storage::<T>()
            .map(|s| s.dense_entities().to_vec())
            .unwrap_or_default()
    }

    fn matches(world: &World, entity: EntityId) -> bool {
        world.has_component::<T>(entity)
    }

    fn fetch<'w>(world: &'w World, entity: EntityId) -> Option<Self::Item<'w>> {
        world.get::<T>(entity)
    }
}

// Implement WorldQuery for Option<&'static T>
impl<T: Component> WorldQuery for Option<&'static T> {
    type Item<'w> = Option<&'w T>;

    fn driver_entities(_world: &World) -> Vec<EntityId> {
        Vec::new() // Option terms are modifiers; they should not act as standalone drivers
    }

    fn matches(_world: &World, _entity: EntityId) -> bool {
        true
    }

    fn fetch<'w>(world: &'w World, entity: EntityId) -> Option<Self::Item<'w>> {
        Some(world.get::<T>(entity))
    }
}

// Implement WorldQuery for With<T>
impl<T: Component> WorldQuery for With<T> {
    type Item<'w> = ();

    fn driver_entities(world: &World) -> Vec<EntityId> {
        world
            .storage::<T>()
            .map(|s| s.dense_entities().to_vec())
            .unwrap_or_default()
    }

    fn matches(world: &World, entity: EntityId) -> bool {
        world.has_component::<T>(entity)
    }

    fn fetch<'w>(_world: &'w World, _entity: EntityId) -> Option<Self::Item<'w>> {
        Some(())
    }
}

// Implement WorldQuery for Without<T>
impl<T: Component> WorldQuery for Without<T> {
    type Item<'w> = ();

    fn driver_entities(_world: &World) -> Vec<EntityId> {
        Vec::new()
    }

    fn matches(world: &World, entity: EntityId) -> bool {
        !world.has_component::<T>(entity)
    }

    fn fetch<'w>(_world: &'w World, _entity: EntityId) -> Option<Self::Item<'w>> {
        Some(())
    }
}

// Tuple implementations (2, 3, 4)
impl<A: WorldQuery, B: WorldQuery> WorldQuery for (A, B) {
    type Item<'w> = (A::Item<'w>, B::Item<'w>);

    fn driver_entities(world: &World) -> Vec<EntityId> {
        let a_driver = A::driver_entities(world);
        if !a_driver.is_empty() {
            return a_driver;
        }
        B::driver_entities(world)
    }

    fn matches(world: &World, entity: EntityId) -> bool {
        A::matches(world, entity) && B::matches(world, entity)
    }

    fn fetch<'w>(world: &'w World, entity: EntityId) -> Option<Self::Item<'w>> {
        let a = A::fetch(world, entity)?;
        let b = B::fetch(world, entity)?;
        Some((a, b))
    }
}

impl<A: WorldQuery, B: WorldQuery, C: WorldQuery> WorldQuery for (A, B, C) {
    type Item<'w> = (A::Item<'w>, B::Item<'w>, C::Item<'w>);

    fn driver_entities(world: &World) -> Vec<EntityId> {
        let a_driver = A::driver_entities(world);
        if !a_driver.is_empty() {
            return a_driver;
        }
        let b_driver = B::driver_entities(world);
        if !b_driver.is_empty() {
            return b_driver;
        }
        C::driver_entities(world)
    }

    fn matches(world: &World, entity: EntityId) -> bool {
        A::matches(world, entity) && B::matches(world, entity) && C::matches(world, entity)
    }

    fn fetch<'w>(world: &'w World, entity: EntityId) -> Option<Self::Item<'w>> {
        let a = A::fetch(world, entity)?;
        let b = B::fetch(world, entity)?;
        let c = C::fetch(world, entity)?;
        Some((a, b, c))
    }
}

impl<A: WorldQuery, B: WorldQuery, C: WorldQuery, D: WorldQuery> WorldQuery for (A, B, C, D) {
    type Item<'w> = (A::Item<'w>, B::Item<'w>, C::Item<'w>, D::Item<'w>);

    fn driver_entities(world: &World) -> Vec<EntityId> {
        let a_driver = A::driver_entities(world);
        if !a_driver.is_empty() {
            return a_driver;
        }
        let b_driver = B::driver_entities(world);
        if !b_driver.is_empty() {
            return b_driver;
        }
        let c_driver = C::driver_entities(world);
        if !c_driver.is_empty() {
            return c_driver;
        }
        D::driver_entities(world)
    }

    fn matches(world: &World, entity: EntityId) -> bool {
        A::matches(world, entity)
            && B::matches(world, entity)
            && C::matches(world, entity)
            && D::matches(world, entity)
    }

    fn fetch<'w>(world: &'w World, entity: EntityId) -> Option<Self::Item<'w>> {
        let a = A::fetch(world, entity)?;
        let b = B::fetch(world, entity)?;
        let c = C::fetch(world, entity)?;
        let d = D::fetch(world, entity)?;
        Some((a, b, c, d))
    }
}
