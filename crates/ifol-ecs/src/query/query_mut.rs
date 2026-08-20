use crate::entity::EntityId;
use crate::query::QueryAccess;
use crate::query::QueryPlanKey;
use crate::world::World;
use std::marker::PhantomData;

#[path = "query_mut_impls.rs"]
mod query_mut_impls;
#[path = "query_mut_tuples.rs"]
mod query_mut_tuples;

/// Extension trait for query signatures that may fetch mutable component data.
///
/// # Safety
///
/// Implementors must ensure that `driver_entities` yields every entity at most
/// once, `matches` agrees with `fetch`, and `fetch` returns references only to
/// the component types declared by `access()`. The declared access must be
/// internally non-aliasing. Violating these invariants can cause undefined
/// behavior because the executor composes results through one mutable `World`
/// pointer.
pub unsafe trait WorldQueryMut: 'static {
    type Item<'w>;

    fn access() -> QueryAccess;
    fn has_driver() -> bool;
    fn driver_entities(world: &World) -> Vec<EntityId>;
    fn matches(world: &World, entity: EntityId) -> bool;

    /// # Safety
    ///
    /// The caller must ensure that every yielded entity is visited at most once
    /// and that the query access has no mutable aliasing.
    unsafe fn fetch<'w>(world: &'w mut World, entity: EntityId) -> Option<Self::Item<'w>>;
}

/// A mutable query over a world.
pub struct QueryMut<'w, Q: WorldQueryMut> {
    world: &'w mut World,
    entities: Vec<EntityId>,
    _marker: PhantomData<Q>,
}

impl<'w, Q: WorldQueryMut> QueryMut<'w, Q> {
    pub(crate) fn new(world: &'w mut World) -> Self {
        let access = Q::access();
        let key = QueryPlanKey::new(
            std::any::TypeId::of::<Q>(),
            access.component_type_ids(),
            world.component_registry().revision(),
            world.structural_version(),
        );
        let entities = world.cached_query_candidates(key, || {
            if Q::has_driver() {
                Q::driver_entities(world)
            } else {
                world.alive_entities()
            }
        });
        Self {
            world,
            entities,
            _marker: PhantomData,
        }
    }

    pub fn iter(&mut self) -> QueryMutIter<'_, Q> {
        let entities = self.entities.clone();
        QueryMutIter {
            world: self.world as *mut World,
            entities: entities.into_iter(),
            _marker: PhantomData,
        }
    }

    pub fn iter_with_entity(&mut self) -> QueryMutEntityIter<'_, Q> {
        QueryMutEntityIter { inner: self.iter() }
    }

    pub fn count(&mut self) -> usize {
        self.iter().count()
    }

    pub fn is_empty(&mut self) -> bool {
        self.count() == 0
    }
}

pub struct QueryMutIter<'w, Q: WorldQueryMut> {
    world: *mut World,
    entities: std::vec::IntoIter<EntityId>,
    _marker: PhantomData<(&'w mut World, Q)>,
}

impl<'w, Q: WorldQueryMut> Iterator for QueryMutIter<'w, Q> {
    type Item = Q::Item<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        for entity in self.entities.by_ref() {
            // SAFETY: the entity list contains each alive entity at most once;
            // mutable aliasing within Q is rejected before QueryMut is created.
            let world: &'w mut World = unsafe { &mut *self.world };
            if world.is_alive(entity) && Q::matches(world, entity) {
                // SAFETY: Q's implementation only returns references to its
                // declared component storages and follows the same invariant.
                if let Some(item) = unsafe { Q::fetch(world, entity) } {
                    return Some(item);
                }
            }
        }
        None
    }
}

pub struct QueryMutEntityIter<'w, Q: WorldQueryMut> {
    inner: QueryMutIter<'w, Q>,
}

impl<'w, Q: WorldQueryMut> Iterator for QueryMutEntityIter<'w, Q> {
    type Item = (EntityId, Q::Item<'w>);

    fn next(&mut self) -> Option<Self::Item> {
        for world_entity in self.inner.entities.by_ref() {
            let world: &'w mut World = unsafe { &mut *self.inner.world };
            if world.is_alive(world_entity)
                && Q::matches(world, world_entity)
                && let Some(item) = unsafe { Q::fetch(world, world_entity) }
            {
                return Some((world_entity, item));
            }
        }
        None
    }
}
