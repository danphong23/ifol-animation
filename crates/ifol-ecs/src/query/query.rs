use crate::entity::EntityId;
use crate::query::WorldQuery;
use crate::world::World;
use std::marker::PhantomData;

/// An executed query over entities in the `World`.
pub struct Query<'w, Q: WorldQuery> {
    world: &'w World,
    _marker: PhantomData<Q>,
}

impl<'w, Q: WorldQuery> Query<'w, Q> {
    /// Creates a new query over the specified world reference.
    pub fn new(world: &'w World) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }

    /// Returns an iterator yielding items for all matching entities.
    pub fn iter(&self) -> impl Iterator<Item = Q::Item<'w>> + 'w {
        let entities = Q::driver_entities(self.world);
        let world = self.world;
        entities.into_iter().filter_map(move |e| {
            if world.is_alive(e) && Q::matches(world, e) {
                Q::fetch(world, e)
            } else {
                None
            }
        })
    }

    /// Returns an iterator yielding `(EntityId, Item)` pairs for all matching entities.
    pub fn iter_with_entity(&self) -> impl Iterator<Item = (EntityId, Q::Item<'w>)> + 'w {
        let entities = Q::driver_entities(self.world);
        let world = self.world;
        entities.into_iter().filter_map(move |e| {
            if world.is_alive(e) && Q::matches(world, e) {
                Q::fetch(world, e).map(|item| (e, item))
            } else {
                None
            }
        })
    }

    /// Counts the total number of matching entities.
    pub fn count(&self) -> usize {
        self.iter().count()
    }

    /// Returns `true` if no entities match the query.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }
}
