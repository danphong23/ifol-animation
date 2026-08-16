use super::extension::dispatch_extension;
use super::validation::RenderGraphValidationError;
use super::{
    encode_compute_commands, encode_copy_command, encode_draw_commands, format_has_stencil,
    RenderGraphExecutor,
};
use crate::api::GpuEngine;
use crate::graph::{DrawAction, RenderGraph, RenderNode, RenderNodePool, RenderTarget};
use crate::resources::handle::{RenderNodeId, TextureHandle};
use crate::resources::ResourceRegistry;

pub(crate) struct TargetViews<'a> {
    pub(crate) color_view: &'a wgpu::TextureView,
    pub(crate) color_format: wgpu::TextureFormat,
    pub(crate) sample_count: u32,
    pub(crate) resolve_view: Option<&'a wgpu::TextureView>,
}

pub(crate) fn resolve_target_views<'a>(
    target: &RenderTarget,
    engine: &'a GpuEngine,
    registry: &'a ResourceRegistry,
    surface_view: Option<&'a wgpu::TextureView>,
) -> Option<TargetViews<'a>> {
    match target {
        RenderTarget::Screen => surface_view
            .zip(engine.surface_format())
            .map(|(view, format)| TargetViews {
                color_view: view,
                color_format: format,
                sample_count: 1,
                resolve_view: None,
            })
            .or_else(|| {
                registry
                    .texture(&TextureHandle(0))
                    .map(|(view, format)| TargetViews {
                        color_view: view,
                        color_format: *format,
                        sample_count: 1,
                        resolve_view: None,
                    })
            }),
        RenderTarget::Offscreen { color, .. } => {
            registry.texture(color).map(|(view, format)| TargetViews {
                color_view: view,
                color_format: *format,
                sample_count: 1,
                resolve_view: None,
            })
        }
        RenderTarget::OffscreenMsaa { color, resolve, .. } => {
            registry.texture(color).and_then(|(color_view, format)| {
                registry
                    .texture(resolve)
                    .map(|(resolve_view, _)| TargetViews {
                        color_view,
                        color_format: *format,
                        sample_count: registry
                            .texture_descriptor(color)
                            .map_or(1, |descriptor| descriptor.sample_count),
                        resolve_view: Some(resolve_view),
                    })
            })
        }
    }
}

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

pub(crate) fn execution_counts_for_graph(
    pool: &RenderNodePool,
    graph: &RenderGraph,
) -> Result<(usize, usize, usize, usize, usize, usize), RenderGraphValidationError> {
    let plan = graph.flatten(pool).map_err(map_graph_flatten_error)?;
    let mut draws = 0;
    let mut computes = 0;
    let mut copies = 0;
    let mut indirect = 0;
    let usages = declared_usage_count(pool, graph);
    for flat_node in &plan.nodes {
        let Some(node) = pool.get(flat_node.node_id) else {
            return Err(RenderGraphValidationError::MissingNode(flat_node.node_id));
        };
        draws += node.commands().len();
        computes += node.compute_commands().len();
        copies += node.copy_commands().len();
        indirect += node
            .commands()
            .iter()
            .filter(|command| {
                matches!(
                    command.action,
                    DrawAction::Indirect { .. } | DrawAction::IndexedIndirect { .. }
                )
            })
            .count();
        indirect += node
            .compute_commands()
            .iter()
            .filter(|command| command.indirect.is_some())
            .count();
    }
    Ok((plan.nodes.len(), draws, computes, copies, indirect, usages))
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

pub(crate) fn declared_usage_count(pool: &RenderNodePool, graph: &RenderGraph) -> usize {
    graph.node_ids.iter().fold(0, |count, node_id| {
        let nested = match pool.get(*node_id) {
            Some(RenderNode::SubGraph { graph: child, .. }) => declared_usage_count(pool, child),
            _ => 0,
        };
        let extension_usage_count = pool
            .get(*node_id)
            .map_or(0, |node| node.extension_usages().len());
        count + graph.resource_usages(node_id).len() + extension_usage_count + nested
    })
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
