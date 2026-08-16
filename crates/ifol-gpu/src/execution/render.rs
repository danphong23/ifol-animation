use std::hash::{Hash, Hasher};

use crate::graph::{DrawAction, DrawCommand};
use crate::graph::{RenderNode, RenderNodePool};
use crate::resources::ResourceRegistry;

use super::validation::{bind_group_slot_index, format_has_stencil, RenderGraphValidationError};

pub(crate) fn bundle_cache_key(
    node: &RenderNode,
    registry: &ResourceRegistry,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    sample_count: u32,
    context_key: u64,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    color_format.hash(&mut hasher);
    depth_format.hash(&mut hasher);
    sample_count.hash(&mut hasher);
    context_key.hash(&mut hasher);
    for command in node.commands() {
        command.pipeline.0.hash(&mut hasher);
        registry
            .pipeline_version(&command.pipeline)
            .hash(&mut hasher);
        for &(slot, bind_group, ref offsets) in &command.bind_groups {
            slot.hash(&mut hasher);
            bind_group.0.hash(&mut hasher);
            registry.bind_group_version(&bind_group).hash(&mut hasher);
            offsets.hash(&mut hasher);
        }
        if let DrawAction::Indexed { mesh, .. } = command.action {
            mesh.0.hash(&mut hasher);
            registry.mesh_version(&mesh).hash(&mut hasher);
        }
    }
    hasher.finish()
}

pub(crate) fn update_render_bundles(
    device: &wgpu::Device,
    pool: &mut RenderNodePool,
    registry: &ResourceRegistry,
    node_ids: &[crate::resources::handle::RenderNodeId],
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    sample_count: u32,
    context_key: u64,
    max_bind_groups: u32,
) -> Result<(), RenderGraphValidationError> {
    for &node_id in node_ids {
        let expected_bundle_key = pool.get(node_id).map(|node| {
            bundle_cache_key(
                node,
                registry,
                color_format,
                depth_format,
                sample_count,
                context_key,
            )
        });
        let Some(node) = pool.get_mut(node_id) else {
            return Err(RenderGraphValidationError::MissingNode(node_id));
        };
        if node.use_bundle()
            && (node.is_dirty()
                || node.bundle().is_none()
                || node.bundle_key() != expected_bundle_key)
        {
            let mut bundle_encoder =
                device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                    label: Some("RenderBundleEncoder"),
                    color_formats: &[Some(color_format)],
                    depth_stencil: depth_format.map(|format| wgpu::RenderBundleDepthStencil {
                        format,
                        depth_read_only: false,
                        stencil_read_only: false,
                    }),
                    sample_count,
                    multiview: None,
                });

            let mut current_pipeline = None;
            let mut current_bind_groups = vec![None; max_bind_groups as usize];
            for command in node.commands() {
                if current_pipeline != Some(command.pipeline) {
                    if let Some(pipeline) = registry.pipeline(&command.pipeline) {
                        bundle_encoder.set_pipeline(pipeline);
                        current_pipeline = Some(command.pipeline);
                    } else {
                        return Err(RenderGraphValidationError::MissingPipeline(
                            command.pipeline,
                        ));
                    }
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
                        bundle_encoder.set_bind_group(slot, group, offsets);
                        current_bind_groups[slot_index] = Some(bind_group);
                    }
                }

                match &command.action {
                    DrawAction::Indexed {
                        mesh,
                        index_range,
                        instance_range,
                    } => {
                        let Some((vertex_buffer, index_buffer, _)) = registry.mesh(mesh) else {
                            return Err(RenderGraphValidationError::MissingMesh(*mesh));
                        };
                        bundle_encoder.set_vertex_buffer(0, vertex_buffer.slice(..));
                        if let Some((index_buffer, format)) = index_buffer {
                            bundle_encoder.set_index_buffer(index_buffer.slice(..), *format);
                            bundle_encoder.draw_indexed(
                                index_range.clone(),
                                0,
                                instance_range.clone(),
                            );
                        } else {
                            bundle_encoder.draw(index_range.clone(), instance_range.clone());
                        }
                    }
                    DrawAction::Procedural {
                        vertex_count,
                        instance_range,
                    } => bundle_encoder.draw(0..*vertex_count, instance_range.clone()),
                    DrawAction::Indirect { buffer, offset } => {
                        let Some(indirect) = registry.buffer(buffer) else {
                            return Err(RenderGraphValidationError::MissingIndirectBuffer(*buffer));
                        };
                        bundle_encoder.draw_indirect(indirect, *offset);
                    }
                    DrawAction::IndexedIndirect {
                        mesh,
                        buffer,
                        offset,
                    } => {
                        let Some((vertex_buffer, Some((index_buffer, format)), _)) =
                            registry.mesh(mesh)
                        else {
                            if !registry.contains_mesh(mesh) {
                                return Err(RenderGraphValidationError::MissingMesh(*mesh));
                            }
                            return Err(RenderGraphValidationError::MissingIndexBuffer(*mesh));
                        };
                        let Some(indirect) = registry.buffer(buffer) else {
                            return Err(RenderGraphValidationError::MissingIndirectBuffer(*buffer));
                        };
                        bundle_encoder.set_vertex_buffer(0, vertex_buffer.slice(..));
                        bundle_encoder.set_index_buffer(index_buffer.slice(..), *format);
                        bundle_encoder.draw_indexed_indirect(indirect, *offset);
                    }
                }
            }

            let bundle = bundle_encoder.finish(&wgpu::RenderBundleDescriptor { label: None });
            match node {
                RenderNode::DrawBatch {
                    bundle: stored_bundle,
                    is_dirty,
                    ..
                }
                | RenderNode::SubGraph {
                    bundle: stored_bundle,
                    is_dirty,
                    ..
                } => {
                    *stored_bundle = Some(bundle);
                    *is_dirty = false;
                }
                RenderNode::ComputeBatch { .. }
                | RenderNode::CopyBatch { .. }
                | RenderNode::Extension { .. } => {
                    unreachable!("non-render node cannot create render bundle")
                }
            }
            node.set_bundle_key(expected_bundle_key.unwrap_or(0));
        }
    }
    Ok(())
}

pub(crate) fn prepare_render_nodes(
    device: &wgpu::Device,
    pool: &mut RenderNodePool,
    registry: &ResourceRegistry,
    ordered_ids: Vec<crate::resources::handle::RenderNodeId>,
    reverse_draw_order: bool,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    sample_count: u32,
    context_key: u64,
    max_bind_groups: u32,
) -> Result<Vec<crate::resources::handle::RenderNodeId>, RenderGraphValidationError> {
    let node_ids = if reverse_draw_order {
        ordered_ids.into_iter().rev().collect()
    } else {
        ordered_ids
    };
    update_render_bundles(
        device,
        pool,
        registry,
        &node_ids,
        color_format,
        depth_format,
        sample_count,
        context_key,
        max_bind_groups,
    )?;
    Ok(node_ids)
}

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
