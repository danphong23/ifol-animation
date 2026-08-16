use crate::graph::{DrawAction, DrawCommand};
use crate::resources::registry::ResourceRegistry;

use super::validation::{bind_group_slot_index, format_has_stencil, RenderGraphValidationError};

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

pub(crate) fn encode_draw_commands(
    render_pass: &mut wgpu::RenderPass<'_>,
    registry: &ResourceRegistry,
    commands: &[DrawCommand],
    max_bind_groups: u32,
) -> Result<(), RenderGraphValidationError> {
    let mut current_pipeline = None;
    let mut current_bind_groups = vec![None; max_bind_groups as usize];
    for command in commands {
        if current_pipeline != Some(command.pipeline) {
            let Some(pipeline) = registry.pipeline(&command.pipeline) else {
                return Err(RenderGraphValidationError::MissingPipeline(
                    command.pipeline,
                ));
            };
            render_pass.set_pipeline(pipeline);
            current_pipeline = Some(command.pipeline);
        }
        for &(slot, bind_group, ref offsets) in &command.bind_groups {
            let Some(slot_index) = bind_group_slot_index(slot, max_bind_groups) else {
                return Err(RenderGraphValidationError::InvalidBindGroupSlot {
                    slot,
                    max_slots: max_bind_groups,
                });
            };
            if current_bind_groups[slot_index] != Some(bind_group) || !offsets.is_empty() {
                let Some(group) = registry.bind_group(&bind_group) else {
                    return Err(RenderGraphValidationError::MissingBindGroup(bind_group));
                };
                render_pass.set_bind_group(slot, group, offsets);
                current_bind_groups[slot_index] = Some(bind_group);
            }
        }
        match &command.action {
            DrawAction::Indexed {
                mesh,
                index_range,
                instance_range,
            } => {
                let Some((vbo, ibo_info, _)) = registry.mesh(mesh) else {
                    return Err(RenderGraphValidationError::MissingMesh(*mesh));
                };
                render_pass.set_vertex_buffer(0, vbo.slice(..));
                if let Some((ibo, format)) = ibo_info {
                    render_pass.set_index_buffer(ibo.slice(..), *format);
                    render_pass.draw_indexed(index_range.clone(), 0, instance_range.clone());
                } else {
                    render_pass.draw(index_range.clone(), instance_range.clone());
                }
            }
            DrawAction::Procedural {
                vertex_count,
                instance_range,
            } => render_pass.draw(0..*vertex_count, instance_range.clone()),
            DrawAction::Indirect { buffer, offset } => {
                let Some(indirect) = registry.buffer(buffer) else {
                    return Err(RenderGraphValidationError::MissingIndirectBuffer(*buffer));
                };
                render_pass.draw_indirect(indirect, *offset);
            }
            DrawAction::IndexedIndirect {
                mesh,
                buffer,
                offset,
            } => {
                let Some((vbo, Some((ibo, format)), _)) = registry.mesh(mesh) else {
                    if !registry.contains_mesh(mesh) {
                        return Err(RenderGraphValidationError::MissingMesh(*mesh));
                    }
                    return Err(RenderGraphValidationError::MissingIndexBuffer(*mesh));
                };
                let Some(indirect) = registry.buffer(buffer) else {
                    return Err(RenderGraphValidationError::MissingIndirectBuffer(*buffer));
                };
                render_pass.set_vertex_buffer(0, vbo.slice(..));
                render_pass.set_index_buffer(ibo.slice(..), *format);
                render_pass.draw_indexed_indirect(indirect, *offset);
            }
        }
    }
    Ok(())
}
