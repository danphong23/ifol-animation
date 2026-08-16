use crate::api::{GpuEngine, TimestampQueryPool};
use crate::graph::{RenderGraph, RenderNodePool};
use crate::memory::SubmissionTracker;
use crate::resources::ResourceRegistry;

use super::counts::execution_counts_for_graph;
use super::flat_compile::compile_flat_graph;
use super::{ExecutionReport, ProfiledExecution, RenderGraphExecutor, RenderGraphProfilingError};

pub(crate) fn execute_timestamped(
    executor: &RenderGraphExecutor,
    engine: &GpuEngine,
    registry: &ResourceRegistry,
    pool: &mut RenderNodePool,
    graph: &RenderGraph,
    surface_view: Option<&wgpu::TextureView>,
    profiler: &mut TimestampQueryPool,
    resolve_buffer: &wgpu::Buffer,
    resolve_offset: u64,
    mut tracker: Option<&mut SubmissionTracker>,
) -> Result<ProfiledExecution, RenderGraphProfilingError> {
    executor.validate_with_device(engine, registry, pool, graph)?;
    let (
        flattened_nodes,
        draw_commands,
        compute_commands,
        copy_commands,
        indirect_commands,
        declared_usages,
    ) = execution_counts_for_graph(pool, graph)?;
    let span = profiler.allocate_span()?;
    let mut encoder = engine
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("RenderGraphProfiledEncoder"),
        });
    profiler.write_span(&mut encoder, span)?;
    compile_flat_graph(
        executor,
        &mut encoder,
        engine,
        pool,
        graph,
        registry,
        surface_view,
    )?;
    profiler.write_span(&mut encoder, span)?;
    profiler.resolve_span(&mut encoder, span, resolve_buffer, resolve_offset)?;
    let tracking_submission = if let Some(tracker) = tracker.as_deref_mut() {
        let submission = tracker.begin();
        profiler.mark_submitted(submission)?;
        Some(submission)
    } else {
        None
    };
    let submission = engine.queue().submit(std::iter::once(encoder.finish()));
    Ok(ProfiledExecution {
        report: ExecutionReport {
            submission,
            flattened_nodes,
            draw_commands,
            compute_commands,
            copy_commands,
            indirect_commands,
            declared_usages,
        },
        span,
        tracking_submission,
    })
}
