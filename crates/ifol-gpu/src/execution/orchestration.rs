use super::extension::dispatch_extension;
use super::targets::{resolve_target_views, TargetViews};
use super::validation::RenderGraphValidationError;
use super::{
    encode_compute_commands, encode_copy_command, encode_draw_commands, format_has_stencil,
    RenderGraphExecutor,
};
use crate::api::GpuEngine;
use crate::graph::{RenderGraph, RenderNode, RenderNodePool};
use crate::resources::handle::RenderNodeId;
use crate::resources::ResourceRegistry;

pub(crate) fn compile_nested_graphs(
    executor: &RenderGraphExecutor,
    encoder: &mut wgpu::CommandEncoder,
    engine: &GpuEngine,
    pool: &mut RenderNodePool,
    graph: &RenderGraph,
    registry: &ResourceRegistry,
    surface_view: Option<&wgpu::TextureView>,
) -> Result<(), RenderGraphValidationError> {
    for &node_id in &graph.node_ids {
        let inner_graph = if let Some(RenderNode::SubGraph { graph: inner, .. }) = pool.get(node_id)
        {
            Some(inner.clone())
        } else {
            None
        };

        if let Some(inner) = inner_graph {
            super::compiler::compile_graph(
                executor,
                encoder,
                engine,
                pool,
                &inner,
                registry,
                surface_view,
            )?;
        }
    }
    Ok(())
}

pub(crate) fn map_graph_flatten_error(
    error: crate::graph::GraphFlattenError,
) -> RenderGraphValidationError {
    match error {
        crate::graph::GraphFlattenError::MissingNode(node) => {
            RenderGraphValidationError::MissingNode(node)
        }
        crate::graph::GraphFlattenError::Cycle(node) => {
            RenderGraphValidationError::DependencyCycle(node)
        }
        crate::graph::GraphFlattenError::DependencyNodeOutsideGraph(node) => {
            RenderGraphValidationError::DependencyOutsideGraph(node)
        }
    }
}

pub(crate) fn owner_graph_for_flat_path<'a>(
    root: &'a RenderGraph,
    pool: &'a RenderNodePool,
    path: &[RenderNodeId],
) -> Result<&'a RenderGraph, RenderGraphValidationError> {
    let mut owner = root;
    for &ancestor_id in path.iter().take(path.len().saturating_sub(1)) {
        let Some(RenderNode::SubGraph { graph, .. }) = pool.get(ancestor_id) else {
            return Err(RenderGraphValidationError::MissingNode(ancestor_id));
        };
        owner = graph;
    }
    Ok(owner)
}

pub(crate) fn flat_plan_owner_path(node: &crate::graph::FlatRenderNode) -> Vec<RenderNodeId> {
    node.path[..node.path.len().saturating_sub(1)].to_vec()
}

pub(crate) fn compile_flat_graph(
    executor: &RenderGraphExecutor,
    encoder: &mut wgpu::CommandEncoder,
    engine: &crate::api::GpuEngine,
    pool: &mut RenderNodePool,
    graph: &RenderGraph,
    registry: &ResourceRegistry,
    surface_view: Option<&wgpu::TextureView>,
) -> Result<(), RenderGraphValidationError> {
    let plan = graph.flatten(pool).map_err(map_graph_flatten_error)?;
    let is_direct_plan = plan.nodes.len() == graph.node_ids.len()
        && plan
            .nodes
            .iter()
            .zip(&graph.node_ids)
            .all(|(flat, direct)| flat.node_id == *direct);
    if is_direct_plan {
        return super::compiler::compile_graph(
            executor,
            encoder,
            engine,
            pool,
            graph,
            registry,
            surface_view,
        );
    }
    let mut last_draw_index = std::collections::HashMap::<Vec<RenderNodeId>, usize>::new();
    for (index, flat_node) in plan.nodes.iter().enumerate() {
        if pool
            .get(flat_node.node_id)
            .is_some_and(|node| !node.commands().is_empty())
        {
            last_draw_index.insert(flat_plan_owner_path(flat_node), index);
        }
    }
    let mut rendered_targets = std::collections::HashSet::<Vec<RenderNodeId>>::new();

    for (index, flat_node) in plan.nodes.iter().enumerate() {
        let Some(node) = pool.get(flat_node.node_id) else {
            return Err(RenderGraphValidationError::MissingNode(flat_node.node_id));
        };
        let owner_path = flat_plan_owner_path(flat_node);
        let owner = owner_graph_for_flat_path(graph, pool, &flat_node.path)?;

        dispatch_extension(executor, encoder, engine, registry, pool, flat_node.node_id)?;
        for command in node.copy_commands() {
            encode_copy_command(encoder, registry, command)?;
        }
        encode_compute_commands(
            encoder,
            registry,
            node.compute_commands(),
            engine.capabilities().max_bind_groups,
        )?;
        if node.commands().is_empty() {
            continue;
        }

        let Some(target_views) =
            resolve_target_views(&owner.target, engine, registry, surface_view)
        else {
            continue;
        };
        let TargetViews {
            color_view,
            color_format,
            sample_count,
            resolve_view,
        } = target_views;
        let depth_stencil_info = owner
            .depth_stencil
            .and_then(|handle| registry.texture(&handle));
        let depth_format = depth_stencil_info.map(|(_, format)| *format);
        let is_first_target_draw = rendered_targets.insert(owner_path.clone());
        let is_last_target_draw = last_draw_index.get(&owner_path).copied() == Some(index);
        let resolve_target = is_last_target_draw.then_some(resolve_view).flatten();
        let load_op = if is_first_target_draw {
            owner
                .clear_color
                .map(|color| {
                    wgpu::LoadOp::Clear(wgpu::Color {
                        r: color[0] as f64,
                        g: color[1] as f64,
                        b: color[2] as f64,
                        a: color[3] as f64,
                    })
                })
                .unwrap_or(wgpu::LoadOp::Load)
        } else {
            wgpu::LoadOp::Load
        };
        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view: color_view,
            depth_slice: None,
            resolve_target,
            ops: wgpu::Operations {
                load: load_op,
                store: wgpu::StoreOp::Store,
            },
        })];
        let depth_stencil_attachment =
            depth_stencil_info.map(|(view, format)| wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: if is_first_target_draw && owner.clear_color.is_some() {
                        wgpu::LoadOp::Clear(1.0)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: format_has_stencil(*format).then_some(wgpu::Operations {
                    load: if is_first_target_draw && owner.clear_color.is_some() {
                        wgpu::LoadOp::Clear(0)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                }),
            });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RenderGraphFlatPass"),
            color_attachments: &color_attachments,
            depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        encode_draw_commands(
            &mut render_pass,
            registry,
            node.commands(),
            engine.capabilities().max_bind_groups,
        )?;
        drop(render_pass);
        let _ = (color_format, depth_format, sample_count);
    }
    Ok(())
}
