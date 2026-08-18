mod support;

use ifol_ecs::query::{With, Without};
use ifol_ecs::{AccessDescriptor, EcsRuntime, EntityId, World};
use support::{Name, OptionalTag, Position, Velocity};

#[test]
fn mutable_query_updates_components_and_preserves_iteration_safety() {
    let mut world = World::new();
    world.insert_world_component(Position { x: 100.0, y: 0.0 });

    let first = world.spawn();
    world.insert(first, Position { x: 1.0, y: 2.0 }).unwrap();
    world.insert(first, Velocity { dx: 3.0, dy: 4.0 }).unwrap();

    let second = world.spawn();
    world.insert(second, Position { x: 10.0, y: 20.0 }).unwrap();
    world
        .insert(second, Velocity { dx: -1.0, dy: 2.0 })
        .unwrap();
    world.insert(second, OptionalTag).unwrap();

    {
        let mut query = world
            .query_mut::<(&'static mut Position, &'static Velocity)>()
            .unwrap();
        for (position, velocity) in query.iter() {
            position.x += velocity.dx;
            position.y += velocity.dy;
        }
    }

    assert_eq!(world.get::<Position>(first).unwrap().x, 4.0);
    assert_eq!(world.get::<Position>(first).unwrap().y, 6.0);
    assert_eq!(world.get::<Position>(second).unwrap().x, 9.0);
    assert_eq!(world.get::<Position>(second).unwrap().y, 22.0);
    assert_eq!(
        world.get_tick::<Position>(first),
        Some(world.current_tick())
    );

    {
        let mut query = world
            .query_mut::<(&'static mut Position, Option<&'static mut OptionalTag>)>()
            .unwrap();
        let mut tagged = 0;
        for (position, tag) in query.iter() {
            if tag.is_some() {
                tagged += 1;
                position.x += 10.0;
            }
        }
        assert_eq!(tagged, 1);
    }

    assert_eq!(world.get::<Position>(second).unwrap().x, 19.0);
}

#[test]
fn mutable_modifier_queries_scan_alive_entities_and_filters() {
    let mut world = World::new();
    world.insert_world_component(Position { x: 0.0, y: 0.0 });
    let entity = world.spawn();
    world.insert(entity, Position { x: 1.0, y: 1.0 }).unwrap();
    world.insert(entity, OptionalTag).unwrap();

    {
        let mut without_name = world.query_mut::<Without<Name>>().unwrap();
        assert_eq!(without_name.count(), 2);
    }

    {
        let mut with_tag = world.query_mut::<With<OptionalTag>>().unwrap();
        assert_eq!(with_tag.count(), 1);
    }

    let mut unit = world.query_mut::<()>().unwrap();
    assert_eq!(unit.iter_with_entity().count(), 2);
    assert!(
        unit.iter_with_entity()
            .all(|(id, ())| id == EntityId::WORLD || id == entity)
    );
}

#[test]
fn mutable_query_rejects_aliasing_signatures() {
    let mut world = World::new();

    assert!(
        world
            .query_mut::<(&'static mut Position, &'static mut Position)>()
            .is_err()
    );
    assert!(
        world
            .query_mut::<(&'static Position, &'static mut Position)>()
            .is_err()
    );
}

#[test]
fn system_context_mutable_query_uses_declared_access() {
    let mut runtime = EcsRuntime::new();
    runtime.register_component::<Position>().unwrap();
    runtime.register_component::<Velocity>().unwrap();
    let phase = ifol_ecs::PhaseId::new("mutable.query");
    runtime.register_phase(phase.clone()).unwrap();

    let mut access = AccessDescriptor::new();
    access.add_read(runtime.world().component_id::<Velocity>().unwrap());
    access.add_write(runtime.world().component_id::<Position>().unwrap());
    let system = runtime
        .register_function_system(
            "MutableQuerySystem",
            |ctx| {
                let mut query = ctx.query_mut::<(&'static mut Position, &'static Velocity)>()?;
                for (position, velocity) in query.iter() {
                    position.x += velocity.dx;
                }
                Ok(())
            },
            access,
            vec![],
        )
        .unwrap();
    runtime.attach_system(&phase, system).unwrap();
    runtime.compile().unwrap();

    let entity = runtime.spawn();
    runtime.insert(entity, Position { x: 1.0, y: 0.0 }).unwrap();
    runtime
        .insert(entity, Velocity { dx: 5.0, dy: 0.0 })
        .unwrap();
    runtime.run_once().unwrap();

    assert_eq!(runtime.get::<Position>(entity).unwrap().x, 6.0);
}
