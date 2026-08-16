use crate::api::GpuEngine;
use crate::graph::RenderNodePool;
use crate::resources::handle::RenderNodeId;
use crate::resources::ResourceRegistry;

use super::compute::encode_compute_commands;
use super::copy::encode_copy_command;
use super::extension::dispatch_extension;
use super::render_pass::{encode_draw_commands, with_render_pass};
use super::{RenderGraphExecutor, RenderGraphValidationError};

pub(crate) fn execute_non_render_nodes(
    executor: &RenderGraphExecutor,
    encoder: &mut wgpu::CommandEncoder,
    engine: &GpuEngine,
    pool: &RenderNodePool,
    registry: &ResourceRegistry,
    node_ids: &[RenderNodeId],
) -> Result<(), RenderGraphValidationError> {
    for &node_id in node_ids {
        let Some(node) = pool.get(node_id) else {
            return Err(RenderGraphValidationError::MissingNode(node_id));
        };
        dispatch_extension(executor, encoder, engine, registry, pool, node_id)?;
        for command in node.copy_commands() {
            encode_copy_command(encoder, registry, command)?;
        }
        encode_compute_commands(
            encoder,
            registry,
            node.compute_commands(),
            engine.capabilities().max_bind_groups,
        )?;
    }
    Ok(())
}

pub(crate) fn execute_graph_prepass(
    executor: &RenderGraphExecutor,
    encoder: &mut wgpu::CommandEncoder,
    engine: &GpuEngine,
    pool: &RenderNodePool,
    registry: &ResourceRegistry,
    node_ids: &[RenderNodeId],
) -> Result<(), RenderGraphValidationError> {
    for &node_id in node_ids {
        let Some(node) = pool.get(node_id) else {
            return Err(RenderGraphValidationError::MissingNode(node_id));
        };
        dispatch_extension(executor, encoder, engine, registry, pool, node_id)?;
        for command in node.copy_commands() {
            encode_copy_command(encoder, registry, command)?;
        }
    }

    for &node_id in node_ids {
        let Some(node) = pool.get(node_id) else {
            return Err(RenderGraphValidationError::MissingNode(node_id));
        };
        encode_compute_commands(
            encoder,
            registry,
            node.compute_commands(),
            engine.capabilities().max_bind_groups,
        )?;
    }
    Ok(())
}

pub(crate) fn execute_ordered_target_nodes(
    executor: &RenderGraphExecutor,
    encoder: &mut wgpu::CommandEncoder,
    engine: &GpuEngine,
    pool: &RenderNodePool,
    registry: &ResourceRegistry,
    node_ids: &[RenderNodeId],
    color_view: &wgpu::TextureView,
    color_format: wgpu::TextureFormat,
    resolve_view: Option<&wgpu::TextureView>,
    depth_stencil_info: Option<(&wgpu::TextureView, wgpu::TextureFormat)>,
    clear_color: Option<[f32; 4]>,
    max_bind_groups: u32,
) -> Result<(), RenderGraphValidationError> {
    let mut rendered_any = false;
    for &node_id in node_ids {
        let Some(node) = pool.get(node_id) else {
            return Err(RenderGraphValidationError::MissingNode(node_id));
        };
        dispatch_extension(executor, encoder, engine, registry, pool, node_id)?;
        for command in node.copy_commands() {
            encode_copy_command(encoder, registry, command)?;
        }
        encode_compute_commands(encoder, registry, node.compute_commands(), max_bind_groups)?;
        if node.commands().is_empty() {
            continue;
        }

        let should_clear = !rendered_any && clear_color.is_some();
        with_render_pass(
            encoder,
            color_view,
            resolve_view,
            depth_stencil_info,
            clear_color
                .map(|color| {
                    wgpu::LoadOp::Clear(wgpu::Color {
                        r: color[0] as f64,
                        g: color[1] as f64,
                        b: color[2] as f64,
                        a: color[3] as f64,
                    })
                })
                .filter(|_| should_clear)
                .unwrap_or(wgpu::LoadOp::Load),
            if should_clear {
                wgpu::LoadOp::Clear(1.0)
            } else {
                wgpu::LoadOp::Load
            },
            if should_clear {
                wgpu::LoadOp::Clear(0)
            } else {
                wgpu::LoadOp::Load
            },
            "RenderGraphSegmentPass",
            |render_pass| {
                encode_draw_commands(render_pass, registry, node.commands(), max_bind_groups)
            },
        )?;
        rendered_any = true;
    }
    let _ = color_format;
    Ok(())
}
