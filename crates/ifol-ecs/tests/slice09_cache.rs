mod support;

use ifol_ecs::EcsRuntime;
use ifol_ecs::schedule::PhaseId;
use ifol_ecs::system::AccessDescriptor;
use support::{Position, Velocity};

#[test]
fn slice09_cache_invalidation_and_recompile_safety() {
    let mut runtime = EcsRuntime::new();

    runtime.register_component::<Position>().unwrap();
    runtime.register_component::<Velocity>().unwrap();

    let p = PhaseId::new("simulate");
    runtime.register_phase(p.clone()).unwrap();

    let sys = runtime
        .register_function_system(
            "MoveSys",
            |ctx| {
                let items: Vec<(ifol_ecs::EntityId, Position)> = ctx
                    .query::<(&'static Position, &'static Velocity)>()?
                    .iter_with_entity()
                    .map(|(e, (pos, vel))| {
                        (
                            e,
                            Position {
                                x: pos.x + vel.dx,
                                y: pos.y + vel.dy,
                            },
                        )
                    })
                    .collect();

                for (e, new_pos) in items {
                    if let Some(pos) = ctx.get_mut::<Position>(e)? {
                        *pos = new_pos;
                    }
                }
                Ok(())
            },
            {
                let mut access = AccessDescriptor::new();
                access.add_read(runtime.world().component_id::<Velocity>().unwrap());
                access.add_write(runtime.world().component_id::<Position>().unwrap());
                access
            },
            vec![],
        )
        .unwrap();

    runtime.attach_system(&p, sys).unwrap();
    runtime.compile().unwrap();

    // Spawn 10 entities
    let mut entities = Vec::new();
    for i in 0..10 {
        let e = runtime.spawn();
        runtime
            .insert(
                e,
                Position {
                    x: i as f32,
                    y: 0.0,
                },
            )
            .unwrap();
        runtime.insert(e, Velocity { dx: 1.0, dy: 0.0 }).unwrap();
        entities.push(e);
    }

    // 1. First execution pass
    let r1 = runtime.run_once().unwrap();
    assert_eq!(r1.execution_revision, 1);
    assert_eq!(
        runtime.get::<Position>(entities[0]),
        Some(&Position { x: 1.0, y: 0.0 })
    );

    // 2. Recompile schedule (e.g. adding a new phase): World data MUST BE PRESERVED!
    let p_post = PhaseId::new("finalize");
    runtime.register_phase(p_post.clone()).unwrap();
    runtime.add_phase_edge(&p, &p_post).unwrap();
    runtime.compile().unwrap();

    // 3. Second execution pass after recompile: World data intact, simulation advances
    let r2 = runtime.run_once().unwrap();
    assert_eq!(r2.execution_revision, 2);
    assert_eq!(
        runtime.get::<Position>(entities[0]),
        Some(&Position { x: 2.0, y: 0.0 })
    );
}

#[test]
fn query_plan_cache_hits_on_data_changes_and_clears_on_structure_changes() {
    let mut runtime = EcsRuntime::new();
    runtime.register_component::<Position>().unwrap();
    let entity = runtime.spawn();
    runtime.insert(entity, Position { x: 1.0, y: 0.0 }).unwrap();

    assert_eq!(runtime.query_plan_cache_stats(), (0, 0));
    assert_eq!(runtime.query::<&'static Position>().count(), 1);
    assert_eq!(runtime.query_plan_cache_stats(), (0, 1));
    assert_eq!(runtime.query::<&'static Position>().count(), 1);
    assert_eq!(runtime.query_plan_cache_stats(), (1, 1));

    runtime.get_mut::<Position>(entity).unwrap().x = 2.0;
    assert_eq!(runtime.query::<&'static Position>().count(), 1);
    assert_eq!(runtime.query_plan_cache_stats(), (2, 1));

    let second = runtime.spawn();
    runtime.insert(second, Position { x: 3.0, y: 0.0 }).unwrap();
    assert_eq!(runtime.query::<&'static Position>().count(), 2);
    assert_eq!(runtime.query_plan_cache_stats(), (2, 2));
}
