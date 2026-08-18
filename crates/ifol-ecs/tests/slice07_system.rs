mod support;

use ifol_ecs::EcsRuntime;
use ifol_ecs::error::SystemError;
use ifol_ecs::schedule::PhaseId;
use ifol_ecs::system::AccessDescriptor;
use support::{FailingSystem, Health};

#[test]
fn slice07_system_context_isolation_and_structured_errors() {
    let mut runtime = EcsRuntime::new();

    runtime.register_component::<Health>().unwrap();
    let phase = PhaseId::new("simulation");
    runtime.register_phase(phase.clone()).unwrap();

    // 1. Register a system that mutates health via SystemContext
    let sys_ok = runtime
        .register_function_system(
            "HealSystem",
            |ctx| {
                let e_candidates: Vec<ifol_ecs::EntityId> = ctx
                    .query::<&'static Health>()
                    .iter_with_entity()
                    .map(|(e, _)| e)
                    .collect();

                for e in e_candidates {
                    if let Some(h) = ctx.get_mut::<Health>(e) {
                        h.0 += 50;
                    }
                }
                Ok(())
            },
            AccessDescriptor::new(),
            vec![],
        )
        .unwrap();

    // 2. Register a system that intentionally returns SystemError
    let sys_fail = runtime
        .register_system(
            "FailingSystem",
            FailingSystem,
            AccessDescriptor::new(),
            vec![],
        )
        .unwrap();

    runtime.attach_system(&phase, sys_ok).unwrap();
    runtime.attach_system(&phase, sys_fail).unwrap();
    runtime.compile().unwrap();

    // Spawn an entity with Health 50
    let e = runtime.spawn();
    runtime.insert(e, Health(50)).unwrap();

    // 3. Execute pass
    let report = runtime.run_once().unwrap();

    // Verify HealSystem ran and updated health
    assert_eq!(report.systems_executed, vec!["HealSystem"]);
    assert_eq!(runtime.get::<Health>(e), Some(&Health(100)));

    // Verify FailingSystem produced structured diagnostic error
    assert_eq!(report.system_errors.len(), 1);
    assert_eq!(report.system_errors[0].0, "FailingSystem");
    assert_eq!(
        report.system_errors[0].1,
        SystemError::new("intentional test failure")
    );
}

#[test]
fn system_ids_are_scoped_to_their_runtime() {
    let mut source = EcsRuntime::new();
    let system_id = source
        .register_function_system("source", |_| Ok(()), AccessDescriptor::new(), vec![])
        .unwrap();

    let mut target = EcsRuntime::new();
    let phase = ifol_ecs::PhaseId::new("target.phase");
    target.register_phase(phase.clone()).unwrap();

    assert!(matches!(
        target.attach_system(&phase, system_id),
        Err(ifol_ecs::EcsError::SystemNotFound(_))
    ));
}
