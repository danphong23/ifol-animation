//! Slice 8 — Dev-Only Test Packages & Multi-Package Integration Pipeline
//!
//! Acceptance criteria:
//! - End-to-end multi-package pipeline execution
//! - Correct topological phase ordering across packages: timeline -> motion -> render
//! - Deterministic state advancement across multiple steps
//! - Zero memory leaks and clean shutdown

mod support;

use ifol_engine::{EngineBuilder, StepInput};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use support::{
    TestMotionPackage, TestRendererPackage, TestTimelinePackage, inline_package,
    inline_package_with_dependency,
};

#[test]
fn multi_package_pipeline_ordering_and_execution() {
    let timeline_counter = Arc::new(AtomicU32::new(0));
    let motion_counter = Arc::new(AtomicU32::new(0));
    let render_counter = Arc::new(AtomicU32::new(0));

    let tc_clone = Arc::clone(&timeline_counter);
    let mc_clone = Arc::clone(&motion_counter);
    let rc_clone = Arc::clone(&render_counter);

    let pkg_timeline = TestTimelinePackage::new();
    let pkg_motion = TestMotionPackage::new();
    let pkg_renderer = TestRendererPackage::new();

    let mut engine = EngineBuilder::new()
        .register_package(inline_package(TestTimelinePackage::id(), move |ctx| {
            pkg_timeline.register(ctx, tc_clone);
        }))
        .register_package(inline_package_with_dependency(
            TestMotionPackage::id(),
            TestTimelinePackage::id(),
            move |ctx| {
                pkg_motion.register(ctx, mc_clone);
            },
        ))
        .register_package(inline_package_with_dependency(
            TestRendererPackage::id(),
            TestMotionPackage::id(),
            move |ctx| {
                pkg_renderer.register(ctx, rc_clone);
            },
        ))
        .build()
        .unwrap();

    // 1. Single step verification
    let report = engine.step(StepInput::default()).unwrap();

    assert_eq!(
        report.ecs_report.phases_visited,
        vec!["timeline", "motion", "render"]
    );
    assert_eq!(
        report.ecs_report.systems_executed,
        vec![
            "timeline_tick_system",
            "motion_integrate_system",
            "render_draw_system"
        ]
    );

    assert_eq!(timeline_counter.load(Ordering::SeqCst), 1);
    assert_eq!(motion_counter.load(Ordering::SeqCst), 1);
    assert_eq!(render_counter.load(Ordering::SeqCst), 1);

    // 2. Multi-step progression (10 frames)
    for _ in 0..9 {
        engine.step(StepInput::default()).unwrap();
    }

    assert_eq!(timeline_counter.load(Ordering::SeqCst), 10);
    assert_eq!(motion_counter.load(Ordering::SeqCst), 10);
    assert_eq!(render_counter.load(Ordering::SeqCst), 10);
    assert_eq!(engine.revision(), 10);

    // 3. Clean shutdown
    let shutdown_rep = engine.shutdown().unwrap();
    assert!(shutdown_rep.clean);
    assert_eq!(shutdown_rep.final_revision, 10);
}
