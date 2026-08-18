use crate::entity::EntityId;
use crate::query::filter::{With, Without};
use crate::storage::Component;
use crate::world::World;
use std::any::TypeId;
use std::collections::HashSet;

/// Type-level access requirements emitted by a query signature.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryAccess {
    pub(crate) reads: Vec<TypeId>,
    pub(crate) writes: Vec<TypeId>,
}

impl QueryAccess {
    fn read<T: Component>() -> Self {
        Self {
            reads: vec![TypeId::of::<T>()],
            writes: Vec::new(),
        }
    }

    pub(crate) fn merge(mut self, other: Self) -> Self {
        self.reads.extend(other.reads);
        self.writes.extend(other.writes);
        self
    }

    pub(crate) fn validate_mutable(&self) -> Result<(), &'static str> {
        let mut writes = HashSet::new();
        for type_id in &self.writes {
            if !writes.insert(*type_id) || self.reads.contains(type_id) {
                return Err("mutable query contains aliased component access");
            }
        }
        Ok(())
    }
}

/// Trait implemented by types that can be fetched via an ECS query.
pub trait WorldQuery: 'static {
    /// The reference type yielded when querying the world for a specific entity.
    type Item<'w>;

    /// Returns the type-level read/write requirements of this query.
    fn access() -> QueryAccess;

    /// Returns whether this query has a required storage that can drive iteration.
    fn has_driver() -> bool;

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

    fn access() -> QueryAccess {
        QueryAccess::read::<T>()
    }

    fn has_driver() -> bool {
        true
    }

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

    fn access() -> QueryAccess {
        QueryAccess::read::<T>()
    }

    fn has_driver() -> bool {
        false
    }

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

    fn access() -> QueryAccess {
        QueryAccess::read::<T>()
    }

    fn has_driver() -> bool {
        true
    }

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

    fn access() -> QueryAccess {
        QueryAccess::read::<T>()
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

    fn fetch<'w>(_world: &'w World, _entity: EntityId) -> Option<Self::Item<'w>> {
        Some(())
    }
}

impl WorldQuery for () {
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

    fn fetch<'w>(_world: &'w World, _entity: EntityId) -> Option<Self::Item<'w>> {
        Some(())
    }
}

macro_rules! impl_tuple_query {
    ($($name:ident),+; $($index:tt),+) => {
        impl<$($name: WorldQuery),+> WorldQuery for ($($name,)+) {
            type Item<'w> = ($($name::Item<'w>,)+);

            fn access() -> QueryAccess {
                QueryAccess::default()$(.merge($name::access()))+
            }

            fn has_driver() -> bool {
                false $(|| $name::has_driver())+
            }

            fn driver_entities(world: &World) -> Vec<EntityId> {
                let mut best: Option<Vec<EntityId>> = None;
                $(
                    if $name::has_driver() {
                        let candidates = $name::driver_entities(world);
                        if best.as_ref().is_none_or(|current| candidates.len() < current.len()) {
                            best = Some(candidates);
                        }
                    }
                )+
                best.unwrap_or_else(|| world.alive_entities())
            }

            fn matches(world: &World, entity: EntityId) -> bool {
                true $(&& $name::matches(world, entity))+
            }

            fn fetch<'w>(world: &'w World, entity: EntityId) -> Option<Self::Item<'w>> {
                Some(($($name::fetch(world, entity)?,)+))
            }
        }
    };
}

impl_tuple_query!(A, B; 0, 1);
impl_tuple_query!(A, B, C; 0, 1, 2);
impl_tuple_query!(A, B, C, D; 0, 1, 2, 3);
impl_tuple_query!(A, B, C, D, E; 0, 1, 2, 3, 4);
impl_tuple_query!(A, B, C, D, E, F; 0, 1, 2, 3, 4, 5);
impl_tuple_query!(A, B, C, D, E, F, G; 0, 1, 2, 3, 4, 5, 6);
impl_tuple_query!(A, B, C, D, E, F, G, H; 0, 1, 2, 3, 4, 5, 6, 7);
