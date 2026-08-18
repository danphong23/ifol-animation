mod support;

use ifol_ecs::EcsRuntime;
use ifol_ecs::schedule::PhaseId;
use ifol_ecs::system::{AccessDescriptor, RunCondition};
use support::{RunCounter, TestConfig};

#[test]
fn slice03_world_singleton_and_run_conditions() {
    let mut runtime = EcsRuntime::new();

    let cfg_id = runtime.register_world_singleton::<TestConfig>().unwrap();
    let _counter_id = runtime.register_world_singleton::<RunCounter>().unwrap();

    let phase = PhaseId::Update;
    runtime.register_phase(phase.clone()).unwrap();

    // 1. Register a system that REQUIRES TestConfig
    let sys_required = runtime
        .register_function_system(
            "ConfigRequiredSystem",
            |ctx| {
                if let Some(counter) = ctx.world_mut::<RunCounter>() {
                    counter.ticks += 10;
                }
                Ok(())
            },
            AccessDescriptor::new(),
            vec![RunCondition::WorldHas(cfg_id, "TestConfig")],
        )
        .unwrap();

    // 2. Register an OPTIONAL system (runs unconditionally)
    let sys_optional = runtime
        .register_function_system(
            "OptionalSystem",
            |ctx| {
                if let Some(counter) = ctx.world_mut::<RunCounter>() {
                    counter.ticks += 1;
                }
                Ok(())
            },
            AccessDescriptor::new(),
            vec![RunCondition::Always],
        )
        .unwrap();

    runtime.attach_system(&phase, sys_required).unwrap();
    runtime.attach_system(&phase, sys_optional).unwrap();
    runtime.compile().unwrap();

    // Insert RunCounter on WORLD_ENTITY, but omit TestConfig
    runtime.insert_world_component(RunCounter { ticks: 0 });

    // 3. First execution pass: ConfigRequiredSystem should be SKIPPED with reason!
    let report1 = runtime.run_once().unwrap();
    assert_eq!(report1.systems_executed, vec!["OptionalSystem"]);
    assert_eq!(report1.systems_skipped.len(), 1);
    assert_eq!(report1.systems_skipped[0].system, "ConfigRequiredSystem");
    assert!(
        report1.systems_skipped[0]
            .reason
            .contains("Missing required world singleton 'TestConfig'")
    );
    assert_eq!(
        runtime.get_world_component::<RunCounter>(),
        Some(&RunCounter { ticks: 1 })
    );

    // 4. Insert TestConfig on WORLD_ENTITY
    runtime.insert_world_component(TestConfig {
        speed_multiplier: 2.0,
        title: "Test Animation".to_string(),
    });

    // 5. Second execution pass: Both systems should execute!
    let report2 = runtime.run_once().unwrap();
    assert_eq!(report2.systems_executed.len(), 2);
    assert_eq!(report2.systems_skipped.len(), 0);
    // counter ticks = 1 (old) + 10 (from required) + 1 (from optional) = 12
    assert_eq!(
        runtime.get_world_component::<RunCounter>(),
        Some(&RunCounter { ticks: 12 })
    );
}
