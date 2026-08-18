use crate::entity::EntityId;
use crate::query::QueryAccess;
use crate::query::filter::{With, Without};
use crate::storage::Component;
use crate::world::World;
use std::marker::PhantomData;

/// Trait implemented by query signatures that may fetch mutable component data.
pub trait WorldQueryMut: 'static {
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

impl<T: Component> WorldQueryMut for &'static mut T {
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
            .storage::<T>()
            .map(|storage| storage.dense_entities().to_vec())
            .unwrap_or_default()
    }

    fn matches(world: &World, entity: EntityId) -> bool {
        world.has_component::<T>(entity)
    }

    unsafe fn fetch<'w>(world: &'w mut World, entity: EntityId) -> Option<Self::Item<'w>> {
        world.get_mut::<T>(entity)
    }
}

impl<T: Component> WorldQueryMut for &'static T {
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
            .storage::<T>()
            .map(|storage| storage.dense_entities().to_vec())
            .unwrap_or_default()
    }

    fn matches(world: &World, entity: EntityId) -> bool {
        world.has_component::<T>(entity)
    }

    unsafe fn fetch<'w>(world: &'w mut World, entity: EntityId) -> Option<Self::Item<'w>> {
        world.get::<T>(entity)
    }
}

impl<T: Component> WorldQueryMut for Option<&'static T> {
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

impl<T: Component> WorldQueryMut for Option<&'static mut T> {
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

impl<T: Component> WorldQueryMut for With<T> {
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
            .storage::<T>()
            .map(|storage| storage.dense_entities().to_vec())
            .unwrap_or_default()
    }

    fn matches(world: &World, entity: EntityId) -> bool {
        world.has_component::<T>(entity)
    }

    unsafe fn fetch<'w>(_world: &'w mut World, _entity: EntityId) -> Option<Self::Item<'w>> {
        Some(())
    }
}

impl<T: Component> WorldQueryMut for Without<T> {
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

impl WorldQueryMut for () {
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

macro_rules! impl_tuple_query_mut {
    ($($name:ident),+) => {
        impl<$($name: WorldQueryMut),+> WorldQueryMut for ($($name,)+) {
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

            unsafe fn fetch<'w>(world: &'w mut World, entity: EntityId) -> Option<Self::Item<'w>> {
                let world_ptr = world as *mut World;
                Some(($((unsafe { $name::fetch(&mut *world_ptr, entity) })?,)+))
            }
        }
    };
}

impl_tuple_query_mut!(A, B);
impl_tuple_query_mut!(A, B, C);
impl_tuple_query_mut!(A, B, C, D);
impl_tuple_query_mut!(A, B, C, D, E);
impl_tuple_query_mut!(A, B, C, D, E, F);
impl_tuple_query_mut!(A, B, C, D, E, F, G);
impl_tuple_query_mut!(A, B, C, D, E, F, G, H);

/// A mutable query over a world.
pub struct QueryMut<'w, Q: WorldQueryMut> {
    world: &'w mut World,
    _marker: PhantomData<Q>,
}

impl<'w, Q: WorldQueryMut> QueryMut<'w, Q> {
    pub(crate) fn new(world: &'w mut World) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }

    pub fn iter(&mut self) -> QueryMutIter<'_, Q> {
        let entities = if Q::has_driver() {
            Q::driver_entities(self.world)
        } else {
            self.world.alive_entities()
        };
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
        let entity = self.inner.entities.next()?;
        let world: &'w mut World = unsafe { &mut *self.inner.world };
        if world.is_alive(entity) && Q::matches(world, entity) {
            let item = unsafe { Q::fetch(world, entity) }?;
            Some((entity, item))
        } else {
            self.next()
        }
    }
}
