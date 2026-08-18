mod support;

use ifol_ecs::query::{QueryAccess, WorldQuery};
use ifol_ecs::{EntityId, World};
use support::Position;

struct PositionPresence;

impl WorldQuery for PositionPresence {
    type Item<'w> = ();

    fn access() -> QueryAccess {
        QueryAccess::read::<Position>()
    }

    fn has_driver() -> bool {
        true
    }

    fn driver_entities(world: &World) -> Vec<EntityId> {
        world
            .component_entities::<Position>()
            .map(|entities| entities.to_vec())
            .unwrap_or_default()
    }

    fn matches(world: &World, entity: EntityId) -> bool {
        world.has_component::<Position>(entity)
    }

    fn fetch<'w>(_world: &'w World, _entity: EntityId) -> Option<Self::Item<'w>> {
        Some(())
    }
}

#[test]
fn external_query_can_publish_a_public_access_contract() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, Position { x: 1.0, y: 2.0 }).unwrap();

    assert_eq!(world.query::<PositionPresence>().count(), 1);

    let access = QueryAccess::read::<Position>().merge(QueryAccess::write::<support::Velocity>());
    assert!(access.validate_mutable().is_ok());
    let mut alias = QueryAccess::read::<Position>();
    alias.add_write::<Position>();
    assert!(alias.validate_mutable().is_err());
}
