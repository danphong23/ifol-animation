mod support;

use ifol_ecs::error::SystemError;
use ifol_ecs::schedule::PhaseId;
use ifol_ecs::system::AccessDescriptor;
use ifol_ecs::{EcsError, EcsRuntime, ExecutionPolicy};
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
                    .query::<&'static Health>()?
                    .iter_with_entity()
                    .map(|(e, _)| e)
                    .collect();

                for e in e_candidates {
                    if let Some(h) = ctx.get_mut::<Health>(e)? {
                        h.0 += 50;
                    }
                }
                Ok(())
            },
            {
                let mut access = AccessDescriptor::new();
                access.add_write(runtime.world().component_id::<Health>().unwrap());
                access
            },
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

#[test]
fn undeclared_component_access_is_reported_as_a_system_error() {
    let mut runtime = EcsRuntime::new();
    runtime.register_component::<Health>().unwrap();
    let phase = ifol_ecs::PhaseId::new("access-check");
    runtime.register_phase(phase.clone()).unwrap();

    let system_id = runtime
        .register_function_system(
            "UndeclaredWriter",
            |ctx| {
                let _ = ctx.get_mut::<Health>(ifol_ecs::EntityId::WORLD)?;
                Ok(())
            },
            AccessDescriptor::new(),
            vec![],
        )
        .unwrap();

    runtime.attach_system(&phase, system_id).unwrap();
    runtime.compile().unwrap();
    let report = runtime.run_once().unwrap();

    assert_eq!(report.system_errors.len(), 1);
    assert!(
        report.system_errors[0]
            .1
            .message
            .contains("write component")
    );
    assert!(
        !report
            .systems_executed
            .contains(&"UndeclaredWriter".to_string())
    );
}

#[test]
fn execution_policy_controls_system_error_flow() {
    let mut runtime = EcsRuntime::new();
    let phase = ifol_ecs::PhaseId::new("policy.stop");
    runtime.register_phase(phase.clone()).unwrap();
    runtime
        .register_system("stop-here", FailingSystem, AccessDescriptor::new(), vec![])
        .and_then(|system| runtime.attach_system(&phase, system).map(|_| system))
        .unwrap();
    let reached = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let reached_clone = reached.clone();
    let next = runtime
        .register_function_system(
            "must-not-run",
            move |_| {
                reached_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            },
            AccessDescriptor::new(),
            vec![],
        )
        .unwrap();
    runtime.attach_system(&phase, next).unwrap();
    runtime.compile().unwrap();
    runtime.set_execution_policy(ExecutionPolicy::StopPhaseOnError);

    let report = runtime.run_once().unwrap();
    assert_eq!(report.system_errors.len(), 1);
    assert_eq!(reached.load(std::sync::atomic::Ordering::SeqCst), 0);

    let mut fail_fast = EcsRuntime::new();
    let fail_phase = ifol_ecs::PhaseId::new("policy.fail-fast");
    fail_fast.register_phase(fail_phase.clone()).unwrap();
    let system = fail_fast
        .register_system("fail-fast", FailingSystem, AccessDescriptor::new(), vec![])
        .unwrap();
    fail_fast.attach_system(&fail_phase, system).unwrap();
    fail_fast.compile().unwrap();
    fail_fast.set_execution_policy(ExecutionPolicy::FailFast);
    assert!(matches!(
        fail_fast.run_once(),
        Err(EcsError::SystemExecutionFailed { .. })
    ));
}
