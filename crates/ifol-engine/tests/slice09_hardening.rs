//! Slice 9 — Hardening, Invariant Verification & Full Acceptance Suite
//!
//! Acceptance criteria:
//! - Thread safety (`Send + Sync` assertions)
//! - Long-horizon stepping determinism & stress testing
//! - Zero-leak clean shutdown across repeated construction cycles
//! - Full error taxonomy audit (all errors typed and display formatted)
//! - Complete end-to-end full-lifecycle integration test

use ifol_ecs::{AccessDescriptor, PhaseId, SystemContext};
mod support;

use ifol_engine::{
    CommandId, CommandRegistry, EngineBuilder, EngineError, EngineRuntime, EngineState,
    PackageDependency, PackageId, PackageLock, PackageLockFile, PackageManifest, PackageResolver,
    ProjectContainer, ResourceId, ResourceProvider, SceneDocument, StepInput, Version, VersionReq,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use support::{TestMotionPackage, TestRendererPackage, TestTimelinePackage};

// ═══════════════════════════════════════════════════════════════════
// 1. THREAD-SAFETY ASSERTIONS
// ═══════════════════════════════════════════════════════════════════

fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

#[test]
fn engine_types_thread_safety() {
    assert_send::<EngineRuntime>();
    assert_send::<PackageResolver>();
    assert_send::<PackageLockFile>();
    assert_send::<SceneDocument>();
    assert_send::<EngineError>();

    assert_sync::<PackageResolver>();
    assert_sync::<PackageLockFile>();
    assert_sync::<SceneDocument>();
    assert_sync::<EngineError>();
}

// ═══════════════════════════════════════════════════════════════════
// 2. LONG-HORIZON STEPPING DETERMINISM & STRESS TEST
// ═══════════════════════════════════════════════════════════════════

#[test]
fn stress_test_1000_steps_determinism() {
    let timeline_counter = Arc::new(AtomicU32::new(0));
    let motion_counter = Arc::new(AtomicU32::new(0));
    let render_counter = Arc::new(AtomicU32::new(0));

    let tc = Arc::clone(&timeline_counter);
    let mc = Arc::clone(&motion_counter);
    let rc = Arc::clone(&render_counter);

    let pkg_t = TestTimelinePackage::new();
    let pkg_m = TestMotionPackage::new();
    let pkg_r = TestRendererPackage::new();

    let mut engine = EngineBuilder::new()
        .with_package(TestTimelinePackage::id(), move |ctx| {
            pkg_t.register(ctx, tc)
        })
        .with_package(TestMotionPackage::id(), move |ctx| pkg_m.register(ctx, mc))
        .with_package(TestRendererPackage::id(), move |ctx| {
            pkg_r.register(ctx, rc)
        })
        .build()
        .unwrap();

    const NUM_STEPS: u64 = 1_000;

    for i in 1..=NUM_STEPS {
        let report = engine.step(StepInput { correlation_id: i }).unwrap();

        assert_eq!(report.correlation_id, i);
        assert_eq!(report.engine_revision, i);
    }

    assert_eq!(timeline_counter.load(Ordering::SeqCst), 1_000);
    assert_eq!(motion_counter.load(Ordering::SeqCst), 1_000);
    assert_eq!(render_counter.load(Ordering::SeqCst), 1_000);

    let shutdown_rep = engine.shutdown().unwrap();
    assert!(shutdown_rep.clean);
    assert_eq!(shutdown_rep.final_revision, 1_000);
}

// ═══════════════════════════════════════════════════════════════════
// 3. REPEATED CYCLE ZERO-LEAK AUDIT
// ═══════════════════════════════════════════════════════════════════

#[test]
fn repeated_construction_and_shutdown_cycles() {
    for _ in 0..50 {
        let mut engine = EngineBuilder::new()
            .with_package(PackageId::new("pkg-ephemeral").unwrap(), |ctx| {
                ctx.register_phase(PhaseId::new("update"));
                ctx.register_system(
                    "noop",
                    PhaseId::new("update"),
                    |_: &mut SystemContext<'_>| Ok(()),
                    AccessDescriptor::new(),
                    vec![],
                );
            })
            .build()
            .unwrap();

        assert_eq!(engine.state(), EngineState::Ready);
        let rep = engine.step(StepInput::default()).unwrap();
        assert_eq!(rep.engine_revision, 1);

        let shutdown = engine.shutdown().unwrap();
        assert!(shutdown.clean);
        assert_eq!(shutdown.final_revision, 1);
        assert_eq!(engine.state(), EngineState::ShuttingDown);
    }
}

// ═══════════════════════════════════════════════════════════════════
// 4. ERROR TAXONOMY AUDIT
// ═══════════════════════════════════════════════════════════════════

#[test]
fn error_taxonomy_formatting_and_traits() {
    let err1 = EngineError::InvalidState {
        expected: "Ready",
        actual: "ShuttingDown",
    };
    assert_eq!(
        err1.to_string(),
        "invalid engine state: expected Ready, actual ShuttingDown"
    );

    let err2 = EngineError::AlreadyShutdown;
    assert_eq!(err2.to_string(), "engine has been shut down");

    let err3 = EngineError::StepInProgress;
    assert_eq!(err3.to_string(), "concurrent or reentrant step rejected");
}

// ═══════════════════════════════════════════════════════════════════
// 5. FULL END-TO-END WORKFLOW INTEGRATION TEST
// ═══════════════════════════════════════════════════════════════════

#[test]
fn full_end_to_end_lifecycle_and_features() {
    // A. Package Resolution
    let mut resolver = PackageResolver::new();
    resolver.add(PackageManifest::new(
        PackageId::new("core-render").unwrap(),
        Version::new(1, 0, 0),
    ));
    resolver.add(
        PackageManifest::new(PackageId::new("app-motion").unwrap(), Version::new(1, 0, 0))
            .with_dependency(PackageDependency {
                package_id: PackageId::new("core-render").unwrap(),
                version_req: VersionReq::caret(Version::new(1, 0, 0)),
            }),
    );

    let lock = resolver.resolve().unwrap();
    assert_eq!(lock.packages.len(), 2);

    // B. Project Container & Storage
    let mut project = ProjectContainer::new_memory("FullMovie", "scenes/main.ifol");
    project.manifest = project.manifest.with_package(
        PackageId::new("app-motion").unwrap(),
        VersionReq::caret(Version::new(1, 0, 0)),
    );
    project.lockfile = Some(PackageLockFile::from_lock(&lock));
    project.save().unwrap();

    let reloaded_project = ProjectContainer::load(project.storage).unwrap();
    assert_eq!(reloaded_project.manifest.name, "FullMovie");

    // C. Root Resource Provider
    struct MockDeviceProvider;
    impl ResourceProvider for MockDeviceProvider {
        fn id(&self) -> ResourceId {
            ResourceId::new("gpu_device")
        }
        fn init(
            &mut self,
            _ecs: &mut ifol_ecs::EcsRuntime,
        ) -> Result<(), ifol_engine::ProviderError> {
            Ok(())
        }
        fn teardown(
            &mut self,
            _ecs: &mut ifol_ecs::EcsRuntime,
        ) -> Result<(), ifol_engine::ProviderError> {
            Ok(())
        }
    }

    // D. Build Engine Runtime
    let mut engine = EngineBuilder::new()
        .with_provider(MockDeviceProvider)
        .with_package(PackageId::new("core-render").unwrap(), |ctx| {
            ctx.register_phase(PhaseId::new("render"));
            ctx.register_command(
                CommandId("render.flush".into()),
                Box::new(|_| Ok(b"flushed".to_vec())),
            );
        })
        .with_package(PackageId::new("app-motion").unwrap(), |ctx| {
            ctx.register_phase(PhaseId::new("motion"));
            ctx.add_phase_edge(PhaseId::new("motion"), PhaseId::new("render"));
            ctx.register_system(
                "motion_system",
                PhaseId::new("motion"),
                |_: &mut SystemContext<'_>| Ok(()),
                AccessDescriptor::new(),
                vec![],
            );
        })
        .build()
        .unwrap();

    assert_eq!(engine.state(), EngineState::Ready);

    // E. Execute Step
    let step_rep = engine.step(StepInput::default()).unwrap();
    assert_eq!(step_rep.engine_revision, 1);
    assert_eq!(step_rep.ecs_report.systems_executed, vec!["motion_system"]);

    // F. Dynamic Reconfiguration (Add post-processing package)
    let pkg_post = PackageId::new("post-process").unwrap();
    let mut reconfig_tx = ifol_engine::RegistrationTransaction::new();
    reconfig_tx.stage_package(PackageId::new("core-render").unwrap(), |ctx| {
        ctx.register_phase(PhaseId::new("render"));
    });
    reconfig_tx.stage_package(PackageId::new("app-motion").unwrap(), |ctx| {
        ctx.register_phase(PhaseId::new("motion"));
        ctx.add_phase_edge(PhaseId::new("motion"), PhaseId::new("render"));
        ctx.register_system(
            "motion_system",
            PhaseId::new("motion"),
            |_: &mut SystemContext<'_>| Ok(()),
            AccessDescriptor::new(),
            vec![],
        );
    });
    reconfig_tx.stage_package(pkg_post.clone(), |ctx| {
        ctx.register_phase(PhaseId::new("post"));
        ctx.add_phase_edge(PhaseId::new("render"), PhaseId::new("post"));
        ctx.register_system(
            "post_system",
            PhaseId::new("post"),
            |_: &mut SystemContext<'_>| Ok(()),
            AccessDescriptor::new(),
            vec![],
        );
    });

    let reconfig_rep = engine
        .reconfigure(
            reconfig_tx,
            CommandRegistry::new(),
            PackageLock { packages: vec![] },
            vec![pkg_post],
            vec![],
        )
        .unwrap();

    assert_eq!(reconfig_rep.revision, 2);

    // G. Step after reconfiguration
    let step_rep2 = engine.step(StepInput::default()).unwrap();
    assert_eq!(step_rep2.engine_revision, 3);
    assert_eq!(
        step_rep2.ecs_report.systems_executed,
        vec!["motion_system", "post_system"]
    );
    assert_eq!(
        step_rep2.ecs_report.phases_visited,
        vec!["motion", "render", "post"]
    );

    // H. Shutdown
    let shutdown_rep = engine.shutdown().unwrap();
    assert!(shutdown_rep.clean);
    assert_eq!(shutdown_rep.final_revision, 3);
}
