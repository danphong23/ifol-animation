use super::{World, WorldQueryMut};
use crate::entity::EntityId;
use crate::query::QueryAccess;

macro_rules! impl_tuple_query_mut {
    ($($name:ident),+) => {
        unsafe impl<$($name: WorldQueryMut),+> WorldQueryMut for ($($name,)+) {
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
