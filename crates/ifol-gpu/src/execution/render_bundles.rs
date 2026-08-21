use crate::graph::{DrawAction, RenderNode, RenderNodePool};
use crate::resources::ResourceRegistry;

use super::bundle_key::bundle_cache_key;
use super::validation::{bind_group_slot_index, RenderGraphValidationError};

#[expect(clippy::too_many_arguments, reason = "render preparation receives independent execution services")]
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

#[expect(clippy::too_many_arguments, reason = "render preparation receives independent execution services")]
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
