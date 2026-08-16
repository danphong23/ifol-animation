use crate::api::GpuEngine;
use crate::graph::{RenderGraph, RenderNodePool};
use crate::resources::ResourceRegistry;

use super::flat_compile::{compile_flat_graph, map_graph_flatten_error};
use super::orchestration::compile_nested_graphs;
use super::targets::resolve_target_views;
use super::render::{encode_graph_render_pass, prepare_render_nodes};
use super::segments::{
    execute_graph_prepass, execute_non_render_nodes, execute_ordered_target_nodes,
};
use super::{RenderGraphExecutor, RenderGraphValidationError};

pub(crate) fn execute_unchecked(
    executor: &RenderGraphExecutor,
    engine: &GpuEngine,
    registry: &ResourceRegistry,
    pool: &mut RenderNodePool,
    graph: &RenderGraph,
    surface_view: Option<&wgpu::TextureView>,
) -> Result<wgpu::SubmissionIndex, RenderGraphValidationError> {
    let mut encoder = engine
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("RenderGraphEncoder"),
        });

    compile_flat_graph(
        executor,
        &mut encoder,
        engine,
        pool,
        graph,
        registry,
        surface_view,
    )?;

    Ok(engine.queue().submit(std::iter::once(encoder.finish())))
}

/// Compile a single graph after nested graphs have been compiled bottom-up.
pub(crate) fn compile_graph(
    executor: &RenderGraphExecutor,
    encoder: &mut wgpu::CommandEncoder,
    engine: &GpuEngine,
    pool: &mut RenderNodePool,
    graph: &RenderGraph,
    registry: &ResourceRegistry,
    surface_view: Option<&wgpu::TextureView>,
) -> Result<(), RenderGraphValidationError> {
    compile_nested_graphs(
        executor,
        encoder,
        engine,
        pool,
        graph,
        registry,
        surface_view,
    )?;

    let ordered_ids = graph
        .ordered_node_ids(pool)
        .map_err(map_graph_flatten_error)?;
    let Some(target_views) = resolve_target_views(&graph.target, engine, registry, surface_view)
    else {
        execute_non_render_nodes(executor, encoder, engine, pool, registry, &ordered_ids)?;
        return Ok(());
    };
    let super::targets::TargetViews {
        color_view,
        color_format,
        sample_count,
        resolve_view,
    } = target_views;

    let depth_stencil_info = graph
        .depth_stencil
        .and_then(|handle| registry.texture(&handle));
    let depth_format = depth_stencil_info.map(|(_, format)| *format);

    let has_draw = ordered_ids.iter().any(|id| {
        pool.get(*id)
            .is_some_and(|node| !node.commands().is_empty())
    });
    let has_non_render = ordered_ids.iter().any(|id| {
        pool.get(*id).is_some_and(|node| {
            !node.copy_commands().is_empty() || !node.compute_commands().is_empty()
        })
    });
    if has_draw && has_non_render {
        execute_ordered_target_nodes(
            executor,
            encoder,
            engine,
            pool,
            registry,
            &ordered_ids,
            color_view,
            color_format,
            resolve_view,
            depth_stencil_info.map(|(view, format)| (view, *format)),
            graph.clear_color,
            engine.capabilities().max_bind_groups,
        )?;
        return Ok(());
    }

    let node_ids = prepare_render_nodes(
        engine.device(),
        pool,
        registry,
        ordered_ids,
        graph.reverse_draw_order,
        color_format,
        depth_format,
        sample_count,
        executor.context_key(),
        engine.capabilities().max_bind_groups,
    )?;
    execute_graph_prepass(executor, encoder, engine, pool, registry, &node_ids)?;
    encode_graph_render_pass(
        encoder,
        pool,
        registry,
        &node_ids,
        color_view,
        resolve_view,
        depth_stencil_info.map(|(view, format)| (view, *format)),
        graph.clear_color,
        engine.capabilities().max_bind_groups,
    )?;
    Ok(())
}
