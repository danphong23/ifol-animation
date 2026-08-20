//! Slice 4 — Root Resource Providers
//!
//! Acceptance criteria:
//! - Root resource provider registration
//! - Topological initialization order (A before B)
//! - Reverse teardown order on shutdown (B before A)
//! - Fail-closed rollback on initialization failure
//! - Cycle and missing dependency detection
//! - Singleton resource registration into ECS

use ifol_ecs::{AccessDescriptor, PhaseId, SystemContext};
use ifol_engine::{
    EngineBuilder, EngineError, PackageId, ProviderError, ResourceId, ResourceProvider, StepInput,
};

mod support;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use support::inline_package;

#[derive(Debug, Clone, PartialEq, Eq)]
struct GpuContext {
    device_id: u32,
}

/// Tracking provider that records calls to init and teardown.
struct TrackingProvider {
    id: ResourceId,
    deps: Vec<ResourceId>,
    log: Arc<std::sync::Mutex<Vec<String>>>,
    fail_on_init: bool,
}

impl ResourceProvider for TrackingProvider {
    fn id(&self) -> ResourceId {
        self.id.clone()
    }

    fn dependencies(&self) -> Vec<ResourceId> {
        self.deps.clone()
    }

    fn init(&mut self, _ecs: &mut ifol_ecs::EcsRuntime) -> Result<(), ProviderError> {
        if self.fail_on_init {
            return Err(ProviderError::InitFailed {
                provider: self.id.to_string(),
                reason: "intentional test failure".into(),
            });
        }
        self.log.lock().unwrap().push(format!("init:{}", self.id));
        Ok(())
    }

    fn teardown(&mut self, _ecs: &mut ifol_ecs::EcsRuntime) -> Result<(), ProviderError> {
        self.log
            .lock()
            .unwrap()
            .push(format!("teardown:{}", self.id));
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 1. TOPOLOGICAL INITIALIZATION & REVERSE TEARDOWN
// ═══════════════════════════════════════════════════════════════════

#[test]
fn topological_init_and_reverse_teardown() {
    let log = Arc::new(std::sync::Mutex::new(Vec::new()));

    // provider_b depends on provider_a
    let prov_b = TrackingProvider {
        id: ResourceId::new("b"),
        deps: vec![ResourceId::new("a")],
        log: Arc::clone(&log),
        fail_on_init: false,
    };

    let prov_a = TrackingProvider {
        id: ResourceId::new("a"),
        deps: vec![],
        log: Arc::clone(&log),
        fail_on_init: false,
    };

    // Add in reverse order: b first, then a
    let mut engine = EngineBuilder::new()
        .with_provider(prov_b)
        .with_provider(prov_a)
        .build()
        .unwrap();

    // Check init order: 'a' MUST be initialized before 'b'
    {
        let entries = log.lock().unwrap().clone();
        assert_eq!(entries, vec!["init:a", "init:b"]);
    }

    // Shutdown engine: 'b' MUST be torn down before 'a'
    engine.shutdown().unwrap();

    {
        let entries = log.lock().unwrap().clone();
        assert_eq!(
            entries,
            vec!["init:a", "init:b", "teardown:b", "teardown:a"]
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// 2. FAIL-CLOSED ROLLBACK ON INIT FAILURE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn rollback_on_mid_chain_init_failure() {
    let log = Arc::new(std::sync::Mutex::new(Vec::new()));

    // 'a' succeeds, 'b' (depends on a) fails
    let prov_a = TrackingProvider {
        id: ResourceId::new("a"),
        deps: vec![],
        log: Arc::clone(&log),
        fail_on_init: false,
    };

    let prov_b = TrackingProvider {
        id: ResourceId::new("b"),
        deps: vec![ResourceId::new("a")],
        log: Arc::clone(&log),
        fail_on_init: true, // FAIL
    };

    let result = EngineBuilder::new()
        .with_provider(prov_a)
        .with_provider(prov_b)
        .build();

    assert!(result.is_err(), "build must fail if provider init fails");

    // 'a' was initialized, then rolled back via teardown:a
    let entries = log.lock().unwrap().clone();
    assert_eq!(entries, vec!["init:a", "teardown:a"]);
}

// ═══════════════════════════════════════════════════════════════════
// 3. CYCLE & DEPENDENCY ERRORS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn provider_cycle_detected() {
    let log = Arc::new(std::sync::Mutex::new(Vec::new()));

    // a -> b -> a
    let prov_a = TrackingProvider {
        id: ResourceId::new("a"),
        deps: vec![ResourceId::new("b")],
        log: Arc::clone(&log),
        fail_on_init: false,
    };

    let prov_b = TrackingProvider {
        id: ResourceId::new("b"),
        deps: vec![ResourceId::new("a")],
        log: Arc::clone(&log),
        fail_on_init: false,
    };

    let result = EngineBuilder::new()
        .with_provider(prov_a)
        .with_provider(prov_b)
        .build();

    assert!(result.is_err());
    match result.unwrap_err() {
        EngineError::Provider(ProviderError::CycleDetected(_)) => {
            // expected
        }
        other => panic!("expected ProviderError::CycleDetected, got {other:?}"),
    }
}

#[test]
fn missing_provider_dependency() {
    let log = Arc::new(std::sync::Mutex::new(Vec::new()));

    let prov_b = TrackingProvider {
        id: ResourceId::new("b"),
        deps: vec![ResourceId::new("non_existent")],
        log: Arc::clone(&log),
        fail_on_init: false,
    };

    let result = EngineBuilder::new().with_provider(prov_b).build();

    assert!(result.is_err());
    match result.unwrap_err() {
        EngineError::Provider(ProviderError::MissingDependency {
            provider,
            dependency,
        }) => {
            assert_eq!(provider, "b");
            assert_eq!(dependency, "non_existent");
        }
        other => panic!("expected MissingDependency, got {other:?}"),
    }
}

#[test]
fn duplicate_provider_id_rejected() {
    let log = Arc::new(std::sync::Mutex::new(Vec::new()));

    let prov1 = TrackingProvider {
        id: ResourceId::new("duplicate"),
        deps: vec![],
        log: Arc::clone(&log),
        fail_on_init: false,
    };
    let prov2 = TrackingProvider {
        id: ResourceId::new("duplicate"),
        deps: vec![],
        log: Arc::clone(&log),
        fail_on_init: false,
    };

    let result = EngineBuilder::new()
        .with_provider(prov1)
        .with_provider(prov2)
        .build();

    assert!(result.is_err());
    match result.unwrap_err() {
        EngineError::Provider(ProviderError::DuplicateProvider(id)) => {
            assert_eq!(id, "duplicate");
        }
        other => panic!("expected DuplicateProvider, got {other:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════
// 4. SINGLETON REGISTRATION VIA PROVIDER
// ═══════════════════════════════════════════════════════════════════

struct SingletonGpuProvider;

impl ResourceProvider for SingletonGpuProvider {
    fn id(&self) -> ResourceId {
        ResourceId::new("gpu_provider")
    }

    fn init(&mut self, ecs: &mut ifol_ecs::EcsRuntime) -> Result<(), ProviderError> {
        ecs.register_world_singleton::<GpuContext>()
            .map_err(|e| ProviderError::InitFailed {
                provider: "gpu_provider".into(),
                reason: format!("{e}"),
            })?;
        ecs.world_mut()
            .insert_world_component(GpuContext { device_id: 101 });
        Ok(())
    }

    fn teardown(&mut self, ecs: &mut ifol_ecs::EcsRuntime) -> Result<(), ProviderError> {
        let _ = ecs.world_mut().remove_world_component::<GpuContext>();
        Ok(())
    }
}

#[test]
fn provider_injects_world_singleton_usable_by_systems() {
    let executed = Arc::new(AtomicUsize::new(0));
    let exec_clone = Arc::clone(&executed);

    let pkg_id = PackageId::new("render-pkg").unwrap();
    let phase = PhaseId::new("render");

    let mut engine = EngineBuilder::new()
        .with_provider(SingletonGpuProvider)
        .register_package(inline_package(pkg_id, move |ctx| {
            ctx.register_phase(phase.clone());
            let exec_inner = Arc::clone(&exec_clone);
            ctx.register_system(
                "gpu_read_system",
                phase.clone(),
                move |ctx: &mut SystemContext<'_>| {
                    exec_inner.fetch_add(1, Ordering::SeqCst);
                    let _ = ctx.system_name();
                    Ok(())
                },
                AccessDescriptor::new(),
                vec![],
            );
        }))
        .build()
        .unwrap();

    let report = engine.step(StepInput::default()).unwrap();
    assert_eq!(report.ecs_report.systems_executed, vec!["gpu_read_system"]);
    assert_eq!(executed.load(Ordering::SeqCst), 1);

    engine.shutdown().unwrap();
}
