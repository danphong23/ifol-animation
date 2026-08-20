use super::World;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Pos {
    x: i32,
    y: i32,
}

#[test]
fn world_spawn_insert_and_despawn() {
    let mut world = World::new();
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.structural_version(), 0);

    let entity = world.spawn();
    assert_eq!(world.structural_version(), 1);

    world.insert(entity, Pos { x: 10, y: 20 }).unwrap();
    assert_eq!(world.structural_version(), 2);
    assert_eq!(world.get::<Pos>(entity), Some(&Pos { x: 10, y: 20 }));

    world.insert(entity, Pos { x: 30, y: 40 }).unwrap();
    assert_eq!(world.structural_version(), 2);
    assert_eq!(world.get::<Pos>(entity), Some(&Pos { x: 30, y: 40 }));

    assert!(world.despawn(entity).is_ok());
    assert_eq!(world.structural_version(), 3);
    assert_eq!(world.get::<Pos>(entity), None);
}
