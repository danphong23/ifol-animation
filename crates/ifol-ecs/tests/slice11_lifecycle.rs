mod support;

use ifol_ecs::EcsRuntime;
use ifol_ecs::schedule::PhaseId;
use ifol_ecs::system::AccessDescriptor;
use std::time::Instant;
use support::{Position, Velocity};

#[test]
fn slice11_runtime_lifecycle_and_100k_stress_test() {
    let mut runtime = EcsRuntime::new();

    runtime.register_component::<Position>().unwrap();
    runtime.register_component::<Velocity>().unwrap();

    let p = PhaseId::Update;
    runtime.register_phase(p.clone()).unwrap();

    let sys = runtime
        .register_function_system(
            "BatchMovementSystem",
            |ctx| {
                let items: Vec<(ifol_ecs::EntityId, Position)> = ctx
                    .query::<(&'static Position, &'static Velocity)>()
                    .iter_with_entity()
                    .map(|(e, (p, v))| {
                        (
                            e,
                            Position {
                                x: p.x + v.dx,
                                y: p.y + v.dy,
                            },
                        )
                    })
                    .collect();

                for (e, p) in items {
                    if let Some(pos) = ctx.get_mut::<Position>(e) {
                        *pos = p;
                    }
                }
                Ok(())
            },
            AccessDescriptor::new(),
            vec![],
        )
        .unwrap();

    runtime.attach_system(&p, sys).unwrap();
    runtime.compile().unwrap();

    // 1. Stress test: Spawn 100,000 entities
    const COUNT: usize = 100_000;
    let start_spawn = Instant::now();
    for i in 0..COUNT {
        let e = runtime.spawn();
        runtime
            .insert(
                e,
                Position {
                    x: i as f32,
                    y: (i * 2) as f32,
                },
            )
            .unwrap();
        runtime.insert(e, Velocity { dx: 1.0, dy: 0.5 }).unwrap();
    }
    let _spawn_duration = start_spawn.elapsed();
    assert_eq!(runtime.world().entity_count(), COUNT + 1); // + WORLD entity

    // 2. Execute pass on 100,000 entities
    let start_tick = Instant::now();
    let report = runtime.run_once().unwrap();
    let _tick_duration = start_tick.elapsed();

    assert_eq!(report.systems_executed, vec!["BatchMovementSystem"]);

    // Verify first and last entity values
    let first_e = ifol_ecs::EntityId::new(1, 1);
    assert_eq!(
        runtime.get::<Position>(first_e),
        Some(&Position { x: 1.0, y: 0.5 })
    );

    let last_e = ifol_ecs::EntityId::new(COUNT as u32, 1);
    assert_eq!(
        runtime.get::<Position>(last_e),
        Some(&Position {
            x: (COUNT - 1) as f32 + 1.0,
            y: ((COUNT - 1) * 2) as f32 + 0.5,
        })
    );

    // 3. Lifecycle: Clear world data
    runtime.clear();
    assert_eq!(runtime.world().entity_count(), 1); // Only WORLD entity remains

    // 4. Lifecycle: Shutdown runtime
    runtime.shutdown();
    assert_eq!(runtime.world().entity_count(), 1);
}
