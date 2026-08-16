use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use super::{
    execution_counts_for_graph, RenderGraphExecutor, RenderGraphProfilingError,
    RenderGraphValidationError,
};
use crate::backend::GpuEngineBuilder;
use crate::graph::{
    ComputeCommand, CopyCommand, DrawAction, DrawCommand, GraphResource, RenderGraph,
    RenderNodePool, RenderTarget, ResourceAccess,
};
use crate::memory::SubmissionTracker;
use crate::resources::{BufferHandle, ComputePipelineHandle, PipelineHandle, ResourceRegistry};

struct CountingDispatcher {
    descriptor: crate::extensions::ExtensionDescriptor,
    calls: Arc<AtomicUsize>,
}

impl crate::extensions::ExtensionDispatcher for CountingDispatcher {
    fn descriptor(&self) -> crate::extensions::ExtensionDescriptor {
        self.descriptor.clone()
    }

    fn encode(
        &self,
        context: crate::extensions::ExtensionExecutionContext<'_, '_>,
    ) -> Result<(), crate::extensions::ExtensionExecutionError> {
        assert_eq!(context.usages().len(), 0);
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[test]
fn extension_without_dispatch_fails_closed_before_resource_lookup() {
    let extension_id = crate::extensions::ExtensionId::new("test.unhandled").unwrap();
    let mut pool = RenderNodePool::new();
    let node = pool.alloc_extension(extension_id.clone(), Vec::new());
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    graph.add_node_id(node);

    assert_eq!(
        RenderGraphExecutor::new().validate(&ResourceRegistry::new(), &pool, &graph),
        Err(RenderGraphValidationError::UnsupportedExtension(
            extension_id
        ))
    );
}

#[test]
fn registered_extension_dispatches_once_in_no_target_path() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let extension_id = crate::extensions::ExtensionId::new("test.counting").unwrap();
    let mut dispatchers = crate::extensions::ExtensionDispatchRegistry::new();
    dispatchers
        .register(Arc::new(CountingDispatcher {
            descriptor: crate::extensions::ExtensionDescriptor {
                id: extension_id.clone(),
                version: 1,
            },
            calls: calls.clone(),
        }))
        .unwrap();

    let mut pool = RenderNodePool::new();
    let node = pool.alloc_extension(extension_id, Vec::new());
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    graph.add_node_id(node);

    RenderGraphExecutor::with_extension_dispatchers(dispatchers)
        .execute_checked(&engine, &ResourceRegistry::new(), &mut pool, &graph)
        .unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[test]
fn profiled_execution_is_opt_in_and_has_typed_backend_fallback() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let Ok(mut profiler) = crate::api::TimestampQueryPool::new(engine.device(), 2) else {
        return;
    };
    let resolve_buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("profiling-resolve-test"),
        size: 16,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let result = RenderGraphExecutor::new().execute_checked_with_timestamp(
        &engine,
        &ResourceRegistry::new(),
        &mut RenderNodePool::new(),
        &RenderGraph::new(RenderTarget::Screen),
        &mut profiler,
        &resolve_buffer,
        0,
    );
    match result {
        Ok(profiled) => assert_eq!(profiled.report.flattened_nodes, 0),
        Err(RenderGraphProfilingError::Profiling(
            crate::api::ProfilingError::UnsupportedEncoderTimestamps,
        )) => {}
        Err(error) => panic!("unexpected profiled execution error: {error:?}"),
    }
}

#[test]
fn tracked_profiled_execution_reserves_pool_until_host_completion() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let Ok(mut profiler) = crate::api::TimestampQueryPool::new(engine.device(), 2) else {
        return;
    };
    let resolve_buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("tracked-profiling-resolve-test"),
        size: 16,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let mut tracker = SubmissionTracker::new();
    let result = RenderGraphExecutor::new().execute_checked_with_timestamp_tracked(
        &engine,
        &ResourceRegistry::new(),
        &mut RenderNodePool::new(),
        &RenderGraph::new(RenderTarget::Screen),
        &mut profiler,
        &resolve_buffer,
        0,
        &mut tracker,
    );
    match result {
        Ok(profiled) => {
            let submission = profiled
                .tracking_submission
                .expect("tracked API must reserve a submission");
            assert_eq!(submission, crate::memory::SubmissionId(1));
            assert_eq!(
                profiler.allocate_span(),
                Err(crate::api::ProfilingError::InFlight)
            );
            assert!(!profiler.reset_after(&tracker).unwrap());
            tracker.mark_completed(submission);
            assert!(profiler.reset_after(&tracker).unwrap());
        }
        Err(RenderGraphProfilingError::Profiling(
            crate::api::ProfilingError::UnsupportedEncoderTimestamps,
        )) => {}
        Err(error) => panic!("unexpected tracked profiling error: {error:?}"),
    }
}

#[test]
fn execution_report_counts_flattened_commands_and_usages() {
    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    let draw = graph.add_batch(
        &mut pool,
        vec![DrawCommand::new(
            PipelineHandle(1),
            DrawAction::Procedural {
                vertex_count: 3,
                instance_range: 0..1,
            },
        )],
    );
    let compute = graph.add_compute_batch(
        &mut pool,
        vec![ComputeCommand::new(ComputePipelineHandle(2), [1, 1, 1])],
    );
    graph.add_copy_batch(
        &mut pool,
        vec![CopyCommand::buffer_to_buffer(
            BufferHandle(3),
            BufferHandle(4),
            16,
        )],
    );
    graph.declare_resource_usage(
        draw,
        GraphResource::Buffer(BufferHandle(5)),
        ResourceAccess::Read,
    );
    graph.declare_resource_usage(
        compute,
        GraphResource::Buffer(BufferHandle(6)),
        ResourceAccess::Write,
    );

    let counts = execution_counts_for_graph(&pool, &graph).unwrap();
    assert_eq!(counts, (3, 1, 1, 1, 0, 2));
}
