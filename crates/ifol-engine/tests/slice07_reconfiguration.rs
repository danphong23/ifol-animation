//! Slice 7 — Dynamic Reconfiguration & Stage-and-Swap
//!
//! Acceptance criteria:
//! - Reconfiguration plan specification
//! - Dynamic package addition/removal and schedule recompilation
//! - Atomic stage-and-swap
//! - Fail-closed guarantee: failed reconfiguration leaves active runtime intact
//! - State guards (must be in Ready)

use ifol_ecs::{AccessDescriptor, PhaseId, SystemContext};
use ifol_engine::{
    CommandRegistry, EngineBuilder, EngineError, EngineState, PackageId, PackageLock,
    RegistrationTransaction, StepInput,
};

#[test]
fn dynamic_reconfiguration_successful_swap() {
    let pkg_a = PackageId::new("pkg-a").unwrap();
    let pkg_b = PackageId::new("pkg-b").unwrap();
    let phase_update = PhaseId::new("update");
    let phase_render = PhaseId::new("render");

    // 1. Initial engine with pkg-a only
    let mut engine = EngineBuilder::new()
        .with_package(pkg_a.clone(), |ctx| {
            ctx.register_phase(phase_update.clone());
            ctx.register_system(
                "sys_a",
                phase_update.clone(),
                |_: &mut SystemContext<'_>| Ok(()),
                AccessDescriptor::new(),
                vec![],
            );
        })
        .build()
        .unwrap();

    let rep1 = engine.step(StepInput::default()).unwrap();
    assert_eq!(rep1.ecs_report.systems_executed, vec!["sys_a"]);

    // 2. Prepare new transaction with pkg-a AND pkg-b
    let mut tx = RegistrationTransaction::new();
    let cmd_reg = CommandRegistry::new();

    tx.stage_package(pkg_a.clone(), |ctx_a| {
        ctx_a.register_phase(phase_update.clone());
        ctx_a.register_system(
            "sys_a",
            phase_update.clone(),
            |_: &mut SystemContext<'_>| Ok(()),
            AccessDescriptor::new(),
            vec![],
        );
    });

    tx.stage_package(pkg_b.clone(), |ctx_b| {
        ctx_b.register_phase(phase_render.clone());
        ctx_b.add_phase_edge(phase_update, phase_render);
        ctx_b.register_system(
            "sys_b",
            PhaseId::new("render"),
            |_: &mut SystemContext<'_>| Ok(()),
            AccessDescriptor::new(),
            vec![],
        );
    });

    let lock = PackageLock { packages: vec![] };

    // 3. Perform dynamic reconfiguration
    let report = engine
        .reconfigure(tx, cmd_reg, lock, vec![pkg_b.clone()], vec![])
        .unwrap();

    assert_eq!(report.added_packages, vec![pkg_b]);
    assert_eq!(engine.state(), EngineState::Ready);

    // 4. Stepping now runs both sys_a and sys_b
    let rep2 = engine.step(StepInput::default()).unwrap();
    assert_eq!(rep2.ecs_report.systems_executed, vec!["sys_a", "sys_b"]);
    assert_eq!(rep2.ecs_report.phases_visited, vec!["update", "render"]);
}

#[test]
fn failed_reconfiguration_preserves_live_runtime() {
    let pkg_a = PackageId::new("pkg-a").unwrap();
    let pkg_bad = PackageId::new("pkg-bad").unwrap();
    let p1 = PhaseId::new("p1");
    let p2 = PhaseId::new("p2");

    // 1. Initial stable engine with pkg-a
    let mut engine = EngineBuilder::new()
        .with_package(pkg_a.clone(), |ctx| {
            ctx.register_phase(p1.clone());
            ctx.register_system(
                "sys_a",
                p1.clone(),
                |_: &mut SystemContext<'_>| Ok(()),
                AccessDescriptor::new(),
                vec![],
            );
        })
        .build()
        .unwrap();

    // 2. Prepare bad transaction (cycle in phase graph)
    let mut bad_tx = RegistrationTransaction::new();
    bad_tx.stage_package(pkg_bad, |ctx_bad| {
        ctx_bad.register_phase(p1.clone());
        ctx_bad.register_phase(p2.clone());
        ctx_bad.add_phase_edge(p1.clone(), p2.clone());
        ctx_bad.add_phase_edge(p2, p1.clone()); // Cycle!
    });

    let lock = PackageLock { packages: vec![] };

    // 3. Attempt reconfiguration -> MUST fail
    let res = engine.reconfigure(
        bad_tx,
        CommandRegistry::new(),
        lock,
        vec![PackageId::new("pkg-bad").unwrap()],
        vec![],
    );
    assert!(res.is_err(), "reconfiguration with cycle must fail");

    // 4. Verify live engine remains 100% functional in Ready state
    assert_eq!(engine.state(), EngineState::Ready);
    let step_rep = engine.step(StepInput::default()).unwrap();
    assert_eq!(step_rep.ecs_report.systems_executed, vec!["sys_a"]);
}

#[test]
fn reconfigure_fails_if_engine_shut_down() {
    let mut engine = EngineBuilder::new().build().unwrap();
    engine.shutdown().unwrap();

    let res = engine.reconfigure(
        RegistrationTransaction::new(),
        CommandRegistry::new(),
        PackageLock { packages: vec![] },
        vec![],
        vec![],
    );

    assert!(matches!(res, Err(EngineError::InvalidState { .. })));
}
