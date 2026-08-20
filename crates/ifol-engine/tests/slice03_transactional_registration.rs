//! Slice 3 — Transactional Registration
//!
//! Acceptance criteria:
//! - Package contribution registration via RegistrationContext
//! - Staging and atomic commit across multiple packages
//! - Fail-closed rollback on duplicate component, invalid phase graph cycle, etc.
//! - Step execution after registration runs systems deterministically
//! - Generic command, query, and event registration

use ifol_ecs::{AccessDescriptor, PhaseId, SystemContext};
use ifol_engine::{
    CommandId, EngineBuilder, EngineError, PackageId, QueryId, StepInput, TransactionError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Position {
    _x: i32,
    _y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Velocity {
    _dx: i32,
    _dy: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct FrameCounter {
    _count: u32,
}

fn phase_update() -> PhaseId {
    PhaseId::new("update")
}

fn phase_render() -> PhaseId {
    PhaseId::new("render")
}

// ═══════════════════════════════════════════════════════════════════
// 1. HAPPY PATH REGISTRATION
// ═══════════════════════════════════════════════════════════════════

#[test]
fn single_package_registration() {
    let pkg_id = PackageId::new("pkg-motion").unwrap();

    let engine = EngineBuilder::new()
        .with_package(pkg_id, |ctx| {
            ctx.register_component::<Position>();
            ctx.register_component::<Velocity>();
            ctx.register_world_singleton::<FrameCounter>();
            ctx.register_phase(phase_update());
            ctx.register_system(
                "motion_system",
                phase_update(),
                |_ctx: &mut SystemContext<'_>| Ok(()),
                AccessDescriptor::new(),
                vec![],
            );
        })
        .build();

    assert!(engine.is_ok(), "registration must succeed");
    let mut engine = engine.unwrap();
    let report = engine.step(StepInput::default()).unwrap();
    assert_eq!(report.ecs_report.systems_executed, vec!["motion_system"]);
}

#[test]
fn multi_package_registration() {
    let pkg_a = PackageId::new("pkg-a").unwrap();
    let pkg_b = PackageId::new("pkg-b").unwrap();

    let engine = EngineBuilder::new()
        .with_package(pkg_a, |ctx| {
            ctx.register_component::<Position>();
            ctx.register_phase(phase_update());
            ctx.register_system(
                "sys_a",
                phase_update(),
                |_: &mut SystemContext<'_>| Ok(()),
                AccessDescriptor::new(),
                vec![],
            );
        })
        .with_package(pkg_b, |ctx| {
            ctx.register_component::<Velocity>();
            ctx.register_phase(phase_render());
            ctx.add_phase_edge(phase_update(), phase_render());
            ctx.register_system(
                "sys_b",
                phase_render(),
                |_: &mut SystemContext<'_>| Ok(()),
                AccessDescriptor::new(),
                vec![],
            );
        })
        .build();

    assert!(engine.is_ok());
    let mut engine = engine.unwrap();
    let report = engine.step(StepInput::default()).unwrap();
    assert_eq!(report.ecs_report.systems_executed, vec!["sys_a", "sys_b"]);
    assert_eq!(report.ecs_report.phases_visited, vec!["update", "render"]);
}

// ═══════════════════════════════════════════════════════════════════
// 2. COMMAND / QUERY / EVENT REGISTRATION
// ═══════════════════════════════════════════════════════════════════

#[test]
fn command_query_event_registration() {
    let pkg_id = PackageId::new("pkg-commands").unwrap();

    let engine = EngineBuilder::new()
        .with_package(pkg_id, |ctx| {
            ctx.register_command(
                CommandId("math.add".into()),
                Box::new(|payload| {
                    if payload.len() >= 2 {
                        Ok(vec![payload[0] + payload[1]])
                    } else {
                        Err("payload too short".into())
                    }
                }),
            );
            ctx.register_query(
                QueryId("status.ping".into()),
                Box::new(|_| Ok(b"pong".to_vec())),
            );
        })
        .build()
        .unwrap();

    let reg = engine.command_registry();
    assert!(reg.has_command(&CommandId("math.add".into())));
    assert!(reg.has_query(&QueryId("status.ping".into())));
    assert_eq!(reg.command_count(), 1);
    assert_eq!(reg.query_count(), 1);
}

// ═══════════════════════════════════════════════════════════════════
// 3. FAIL-CLOSED TRANSACTION ROLLBACK
// ═══════════════════════════════════════════════════════════════════

#[test]
fn cycle_in_phase_graph_aborts_build() {
    let pkg_id = PackageId::new("pkg-cycle").unwrap();
    let p1 = PhaseId::new("phase1");
    let p2 = PhaseId::new("phase2");

    let result = EngineBuilder::new()
        .with_package(pkg_id, |ctx| {
            ctx.register_phase(p1.clone());
            ctx.register_phase(p2.clone());
            ctx.add_phase_edge(p1.clone(), p2.clone());
            ctx.add_phase_edge(p2, p1); // cycle!
        })
        .build();

    assert!(result.is_err(), "build with phase cycle must fail");
    match result.unwrap_err() {
        EngineError::Registration(TransactionError::ContributionFailed { reason, .. }) => {
            assert!(
                reason.contains("cycle") || reason.contains("DAG") || reason.contains("Cycle"),
                "expected cycle error, got {reason}"
            );
        }
        EngineError::Registration(TransactionError::Ecs(
            ifol_ecs::EcsError::PhaseCycleDetected(_),
        ))
        | EngineError::Ecs(ifol_ecs::EcsError::PhaseCycleDetected(_)) => {
            // also acceptable
        }
        other => panic!("expected Registration/Ecs cycle error, got {other:?}"),
    }
}

#[test]
fn duplicate_command_aborts_build() {
    let pkg_a = PackageId::new("pkg-a").unwrap();
    let pkg_b = PackageId::new("pkg-b").unwrap();
    let cmd_id = CommandId("duplicate.cmd".into());

    let result = EngineBuilder::new()
        .with_package(pkg_a, |ctx| {
            ctx.register_command(cmd_id.clone(), Box::new(|_| Ok(vec![])));
        })
        .with_package(pkg_b, |ctx| {
            ctx.register_command(cmd_id.clone(), Box::new(|_| Ok(vec![])));
        })
        .build();

    assert!(result.is_err(), "build with duplicate command must fail");
    match result.unwrap_err() {
        EngineError::Registration(TransactionError::ContributionFailed { package, reason }) => {
            assert_eq!(package.as_str(), "pkg-b");
            assert!(reason.contains("duplicate command ID"));
        }
        other => panic!("expected Registration error, got {other:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════
// 4. DETERMINISTIC EXECUTION
// ═══════════════════════════════════════════════════════════════════

#[test]
fn multi_step_execution_with_state() {
    let pkg_id = PackageId::new("pkg-counter").unwrap();

    let mut engine = EngineBuilder::new()
        .with_package(pkg_id, |ctx| {
            ctx.register_phase(phase_update());
            ctx.register_system(
                "tick",
                phase_update(),
                |_: &mut SystemContext<'_>| Ok(()),
                AccessDescriptor::new(),
                vec![],
            );
        })
        .build()
        .unwrap();

    for i in 1..=5 {
        let rep = engine.step(StepInput::default()).unwrap();
        assert_eq!(rep.engine_revision, i);
        assert_eq!(rep.ecs_report.systems_executed, vec!["tick"]);
    }
}
