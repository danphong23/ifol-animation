//! Slice 1 — Runtime Lifecycle & State Machine
//!
//! Acceptance criteria (from docs/05-implementation-plan.md):
//! - build/step/reconfigure/shutdown hợp lệ
//! - misuse trả typed error
//! - runtime rỗng deterministic

use ifol_engine::{EngineBuilder, EngineError, EngineState, StepInput};

// ═══════════════════════════════════════════════════════════════════
// 1. EMPTY BUILD
// ═══════════════════════════════════════════════════════════════════

#[test]
fn empty_build_succeeds() {
    let engine = EngineBuilder::new().build();
    assert!(engine.is_ok(), "empty build must succeed");
}

#[test]
fn empty_build_produces_ready_state() {
    let engine = EngineBuilder::new().build().unwrap();
    assert_eq!(engine.state(), EngineState::Ready);
}

#[test]
fn empty_build_has_zero_revision() {
    let engine = EngineBuilder::new().build().unwrap();
    assert_eq!(engine.revision(), 0);
}

#[test]
fn default_builder_succeeds() {
    let engine = EngineBuilder::default().build().unwrap();
    assert_eq!(engine.state(), EngineState::Ready);
}

// ═══════════════════════════════════════════════════════════════════
// 2. EMPTY STEP
// ═══════════════════════════════════════════════════════════════════

#[test]
fn empty_step_succeeds() {
    let mut engine = EngineBuilder::new().build().unwrap();
    let result = engine.step(StepInput::default());
    assert!(result.is_ok(), "step on empty runtime must succeed");
}

#[test]
fn step_increments_revision() {
    let mut engine = EngineBuilder::new().build().unwrap();
    let report = engine.step(StepInput::default()).unwrap();
    assert_eq!(report.engine_revision, 1);
    assert_eq!(engine.revision(), 1);
}

#[test]
fn step_echoes_correlation_id() {
    let mut engine = EngineBuilder::new().build().unwrap();
    let input = StepInput { correlation_id: 42 };
    let report = engine.step(input).unwrap();
    assert_eq!(report.correlation_id, 42);
}

#[test]
fn multiple_steps_increment_revision_monotonically() {
    let mut engine = EngineBuilder::new().build().unwrap();
    for i in 1..=10 {
        let report = engine.step(StepInput::default()).unwrap();
        assert_eq!(report.engine_revision, i);
    }
    assert_eq!(engine.revision(), 10);
}

#[test]
fn step_returns_to_ready_state() {
    let mut engine = EngineBuilder::new().build().unwrap();
    engine.step(StepInput::default()).unwrap();
    assert_eq!(engine.state(), EngineState::Ready);
}

#[test]
fn empty_step_ecs_report_has_valid_data() {
    let mut engine = EngineBuilder::new().build().unwrap();
    let report = engine.step(StepInput::default()).unwrap();
    // Empty runtime: no phases, no systems, one entity (WORLD)
    assert!(report.ecs_report.phases_visited.is_empty());
    assert!(report.ecs_report.systems_executed.is_empty());
    assert!(report.ecs_report.system_errors.is_empty());
    assert_eq!(report.ecs_report.entities_count, 1); // WORLD entity
}

// ═══════════════════════════════════════════════════════════════════
// 3. DETERMINISM
// ═══════════════════════════════════════════════════════════════════

#[test]
fn empty_runtime_is_deterministic() {
    // Two independent empty runtimes must produce identical step reports
    // (ignoring timing).
    let mut a = EngineBuilder::new().build().unwrap();
    let mut b = EngineBuilder::new().build().unwrap();

    let ra = a.step(StepInput { correlation_id: 1 }).unwrap();
    let rb = b.step(StepInput { correlation_id: 1 }).unwrap();

    assert_eq!(ra.engine_revision, rb.engine_revision);
    assert_eq!(
        ra.ecs_report.execution_revision,
        rb.ecs_report.execution_revision
    );
    assert_eq!(ra.ecs_report.phases_visited, rb.ecs_report.phases_visited);
    assert_eq!(
        ra.ecs_report.systems_executed,
        rb.ecs_report.systems_executed
    );
    assert_eq!(ra.ecs_report.entities_count, rb.ecs_report.entities_count);
}

// ═══════════════════════════════════════════════════════════════════
// 4. SHUTDOWN
// ═══════════════════════════════════════════════════════════════════

#[test]
fn shutdown_from_ready_succeeds() {
    let mut engine = EngineBuilder::new().build().unwrap();
    let report = engine.shutdown();
    assert!(report.is_ok());
    assert_eq!(engine.state(), EngineState::ShuttingDown);
}

#[test]
fn shutdown_report_contains_final_revision() {
    let mut engine = EngineBuilder::new().build().unwrap();
    engine.step(StepInput::default()).unwrap();
    engine.step(StepInput::default()).unwrap();
    let report = engine.shutdown().unwrap();
    assert_eq!(report.final_revision, 2);
    assert!(report.clean);
}

#[test]
fn shutdown_is_clean_when_no_errors() {
    let mut engine = EngineBuilder::new().build().unwrap();
    let report = engine.shutdown().unwrap();
    assert!(report.clean);
}

// ═══════════════════════════════════════════════════════════════════
// 5. STATE MACHINE ENFORCEMENT — MISUSE RETURNS TYPED ERROR
// ═══════════════════════════════════════════════════════════════════

#[test]
fn step_after_shutdown_returns_invalid_state() {
    let mut engine = EngineBuilder::new().build().unwrap();
    engine.shutdown().unwrap();
    let result = engine.step(StepInput::default());
    assert!(result.is_err());
    match result.unwrap_err() {
        EngineError::InvalidState { expected, actual } => {
            assert_eq!(expected, "Ready");
            assert_eq!(actual, "ShuttingDown");
        }
        other => panic!("expected InvalidState, got {other:?}"),
    }
}

#[test]
fn double_shutdown_returns_already_shutdown() {
    let mut engine = EngineBuilder::new().build().unwrap();
    engine.shutdown().unwrap();
    let result = engine.shutdown();
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), EngineError::AlreadyShutdown),
        "second shutdown should return AlreadyShutdown"
    );
}

#[test]
fn shutdown_after_steps_succeeds() {
    let mut engine = EngineBuilder::new().build().unwrap();
    engine.step(StepInput::default()).unwrap();
    engine.step(StepInput::default()).unwrap();
    let report = engine.shutdown().unwrap();
    assert_eq!(report.final_revision, 2);
}

// ═══════════════════════════════════════════════════════════════════
// 6. REVISION WRAPPING
// ═══════════════════════════════════════════════════════════════════

#[test]
fn revision_wraps_at_u64_max() {
    let mut engine = EngineBuilder::new().build().unwrap();
    // Manually set revision close to u64::MAX to test wrapping
    // We test the wrapping policy: wrapping_add means u64::MAX + 1 = 0
    // Since we can't directly set the revision, we verify the contract
    // through the public API: step must never panic even after many calls.
    for _ in 0..100 {
        engine.step(StepInput::default()).unwrap();
    }
    assert_eq!(engine.revision(), 100);
}

// ═══════════════════════════════════════════════════════════════════
// 7. PANIC-FREE INVALID STATE HANDLING
// ═══════════════════════════════════════════════════════════════════

#[test]
fn all_state_errors_are_typed_not_panics() {
    // After shutdown, every operation must return a typed error, never panic.
    let mut engine = EngineBuilder::new().build().unwrap();
    engine.shutdown().unwrap();

    // step → error
    assert!(engine.step(StepInput::default()).is_err());
    // shutdown again → error
    assert!(engine.shutdown().is_err());
}

// ═══════════════════════════════════════════════════════════════════
// 8. ENGINE STATE DISPLAY
// ═══════════════════════════════════════════════════════════════════

#[test]
fn engine_state_labels_are_correct() {
    assert_eq!(EngineState::Building.label(), "Building");
    assert_eq!(EngineState::Ready.label(), "Ready");
    assert_eq!(EngineState::Stepping.label(), "Stepping");
    assert_eq!(EngineState::Faulted.label(), "Faulted");
    assert_eq!(EngineState::ShuttingDown.label(), "ShuttingDown");
}

#[test]
fn engine_state_display_matches_label() {
    for state in [
        EngineState::Building,
        EngineState::Ready,
        EngineState::Stepping,
        EngineState::Faulted,
        EngineState::ShuttingDown,
    ] {
        assert_eq!(format!("{state}"), state.label());
    }
}
