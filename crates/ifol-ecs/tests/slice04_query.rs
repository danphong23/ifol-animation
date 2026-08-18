mod support;

use ifol_ecs::World;
use ifol_ecs::query::{With, Without};
use support::{Health, Name, OptionalTag, Position, Velocity};

#[test]
fn slice04_query_tuples_filters_and_world_entity() {
    let mut world = World::new();

    // 1. Insert component onto WORLD_ENTITY
    world.insert_world_component(Position { x: 0.0, y: 0.0 });

    // 2. Spawn 10 normal entities with Position, Velocity, Health, and OptionalTag
    for i in 1..=10 {
        let e = world.spawn();
        world
            .insert(
                e,
                Position {
                    x: i as f32,
                    y: i as f32,
                },
            )
            .unwrap();
        world.insert(e, Velocity { dx: 1.0, dy: 1.0 }).unwrap();
        world.insert(e, Health(100)).unwrap();
        if i % 2 == 0 {
            world.insert(e, OptionalTag).unwrap();
        }
    }

    // 3. Query<&Position> includes normal entities + WORLD_ENTITY (total = 11)
    let pos_query = world.query::<&'static Position>();
    assert_eq!(pos_query.count(), 11);

    // 4. Query<(&Position, &Velocity)>: WORLD_ENTITY does not have Velocity, so only 10 normal entities match
    let dual_query = world.query::<(&'static Position, &'static Velocity)>();
    assert_eq!(dual_query.count(), 10);

    // 5. Query with filters: With<OptionalTag>
    let tagged_query = world.query::<(&'static Position, With<OptionalTag>)>();
    assert_eq!(tagged_query.count(), 5);

    // 6. Query with filters: Without<OptionalTag> (5 normal + 1 WORLD_ENTITY = 6)
    let untagged_query = world.query::<(&'static Position, Without<OptionalTag>)>();
    assert_eq!(untagged_query.count(), 6);

    // 7. Query with Option<&OptionalTag>
    let optional_query = world.query::<(&'static Position, Option<&'static OptionalTag>)>();
    assert_eq!(optional_query.count(), 11);

    // 8. Query 0 match: No entity has Name component
    let empty_query = world.query::<&'static Name>();
    assert_eq!(empty_query.count(), 0);
    assert!(empty_query.is_empty());
}
