mod support;

use ifol_ecs::EcsRuntime;
use ifol_ecs::schedule::PhaseId;
use ifol_ecs::system::AccessDescriptor;
use support::{Health, Position};

#[test]
fn slice08_execute_deferred_commands_and_safe_points() {
    let mut runtime = EcsRuntime::new();

    runtime.register_component::<Position>().unwrap();
    runtime.register_component::<Health>().unwrap();

    let p1 = PhaseId::PreUpdate;
    let p2 = PhaseId::Update;
    runtime.register_phase(p1.clone()).unwrap();
    runtime.register_phase(p2.clone()).unwrap();
    runtime.add_phase_edge(&p1, &p2).unwrap();

    // System 1 (PreUpdate): Iterates entities, queues deferred despawn and deferred insert
    let sys1 = runtime
        .register_function_system(
            "DeferredSpawnerSystem",
            |ctx| {
                let entities: Vec<ifol_ecs::EntityId> = ctx
                    .query::<&'static Position>()
                    .iter_with_entity()
                    .map(|(e, _)| e)
                    .collect();

                for e in entities {
                    // Queue deferred insert of Health component
                    ctx.commands().insert(e, Health(200));
                }
                Ok(())
            },
            AccessDescriptor::new(),
            vec![],
        )
        .unwrap();

    // System 2 (Update): Reads Health component inserted at safe point!
    let sys2 = runtime
        .register_function_system(
            "HealthReaderSystem",
            |ctx| {
                let healths: Vec<i32> =
                    ctx.query::<&'static Health>().iter().map(|h| h.0).collect();

                assert_eq!(healths, vec![200]); // Health is present because commands were flushed at safe point!
                Ok(())
            },
            AccessDescriptor::new(),
            vec![],
        )
        .unwrap();

    runtime.attach_system(&p1, sys1).unwrap();
    runtime.attach_system(&p2, sys2).unwrap();
    runtime.compile().unwrap();

    // Spawn 1 entity with Position
    let e = runtime.spawn();
    runtime.insert(e, Position { x: 10.0, y: 10.0 }).unwrap();

    // Execute pass
    let report = runtime.run_once().unwrap();

    assert_eq!(report.phases_visited.len(), 2);
    assert_eq!(report.systems_executed.len(), 2);
    assert!(report.commands_processed >= 1);
    assert_eq!(runtime.get::<Health>(e), Some(&Health(200)));
}
