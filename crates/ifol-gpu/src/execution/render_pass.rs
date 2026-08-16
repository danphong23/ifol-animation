use crate::graph::RenderNodePool;
use crate::resources::ResourceRegistry;

use super::validation::{format_has_stencil, RenderGraphValidationError};

#[path = "draw.rs"]
mod draw;
pub(crate) use draw::encode_draw_commands;

pub(crate) fn with_render_pass<T>(
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    resolve_view: Option<&wgpu::TextureView>,
    depth_stencil_info: Option<(&wgpu::TextureView, wgpu::TextureFormat)>,
    color_load: wgpu::LoadOp<wgpu::Color>,
    depth_load: wgpu::LoadOp<f32>,
    stencil_load: wgpu::LoadOp<u32>,
    label: &'static str,
    encode: impl FnOnce(&mut wgpu::RenderPass<'_>) -> Result<T, RenderGraphValidationError>,
) -> Result<T, RenderGraphValidationError> {
    let color_attachments = [Some(wgpu::RenderPassColorAttachment {
        view: color_view,
        depth_slice: None,
        resolve_target: resolve_view,
        ops: wgpu::Operations {
            load: color_load,
            store: wgpu::StoreOp::Store,
        },
    })];
    let depth_stencil_attachment =
        depth_stencil_info.map(|(view, format)| wgpu::RenderPassDepthStencilAttachment {
            view,
            depth_ops: Some(wgpu::Operations {
                load: depth_load,
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: format_has_stencil(format).then_some(wgpu::Operations {
                load: stencil_load,
                store: wgpu::StoreOp::Store,
            }),
        });
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &color_attachments,
        depth_stencil_attachment,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    encode(&mut render_pass)
}

pub(crate) fn encode_graph_render_pass(
    encoder: &mut wgpu::CommandEncoder,
    pool: &RenderNodePool,
    registry: &ResourceRegistry,
    node_ids: &[crate::resources::handle::RenderNodeId],
    color_view: &wgpu::TextureView,
    resolve_view: Option<&wgpu::TextureView>,
    depth_stencil_info: Option<(&wgpu::TextureView, wgpu::TextureFormat)>,
    clear_color: Option<[f32; 4]>,
    max_bind_groups: u32,
) -> Result<(), RenderGraphValidationError> {
    let load_op = clear_color
        .map(|color| {
            wgpu::LoadOp::Clear(wgpu::Color {
                r: color[0] as f64,
                g: color[1] as f64,
                b: color[2] as f64,
                a: color[3] as f64,
            })
        })
        .unwrap_or(wgpu::LoadOp::Load);

    with_render_pass(
        encoder,
        color_view,
        resolve_view,
        depth_stencil_info,
        load_op,
        if clear_color.is_some() {
            wgpu::LoadOp::Clear(1.0)
        } else {
            wgpu::LoadOp::Load
        },
        if clear_color.is_some() {
            wgpu::LoadOp::Clear(0)
        } else {
            wgpu::LoadOp::Load
        },
        "RenderGraphPass",
        |render_pass| {
            for &node_id in node_ids {
                let Some(node) = pool.get(node_id) else {
                    return Err(RenderGraphValidationError::MissingNode(node_id));
                };
                if node.use_bundle() {
                    if let Some(bundle) = node.bundle() {
                        render_pass.execute_bundles(std::iter::once(bundle));
                    }
                } else {
                    encode_draw_commands(render_pass, registry, node.commands(), max_bind_groups)?;
                }
            }
            Ok(())
        },
    )?;
    Ok(())
}
