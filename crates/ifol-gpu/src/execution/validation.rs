pub use super::validation_errors::RenderGraphValidationError;
use crate::extensions::ExtensionDispatchRegistry;
use crate::graph::{
    CopyCommand, DrawAction, GraphResource, RenderGraph, RenderNode, RenderNodePool, TextureAspect,
};
use crate::resources::registry::ResourceRegistry;

#[cfg(test)]
pub(crate) use super::validation_copy::texture_supports_aspect;
pub(crate) use super::validation_copy::{
    format_has_stencil, validate_copy_range, validate_indirect_buffer, validate_texture_copy,
};
pub(crate) use super::validation_layout::{
    bind_group_slot_index, validate_bind_group_offsets, validate_compute_pipeline_layout,
    validate_render_pipeline_layout,
};
use super::validation_target::{validate_depth_stencil, validate_render_target};

pub(crate) fn validate_graph(
    registry: &ResourceRegistry,
    pool: &RenderNodePool,
    graph: &RenderGraph,
    max_bind_groups: u32,
    extension_dispatchers: &ExtensionDispatchRegistry,
) -> Result<(), RenderGraphValidationError> {
    graph.flatten(pool).map_err(|error| match error {
        crate::graph::GraphFlattenError::MissingNode(node) => {
            RenderGraphValidationError::MissingNode(node)
        }
        crate::graph::GraphFlattenError::Cycle(node) => {
            RenderGraphValidationError::DependencyCycle(node)
        }
        crate::graph::GraphFlattenError::DependencyNodeOutsideGraph(node) => {
            RenderGraphValidationError::DependencyOutsideGraph(node)
        }
    })?;

    let target_sample_count = validate_render_target(registry, &graph.target)?;
    validate_depth_stencil(registry, graph.depth_stencil, target_sample_count)?;

    for &node_id in &graph.node_ids {
        let node = pool
            .get(node_id)
            .ok_or(RenderGraphValidationError::MissingNode(node_id))?;
        if let RenderNode::Extension { extension, usages } = node {
            let Some(dispatcher) = extension_dispatchers.get(extension) else {
                return Err(RenderGraphValidationError::UnsupportedExtension(
                    extension.clone(),
                ));
            };
            dispatcher.validate(usages).map_err(|error| {
                RenderGraphValidationError::ExtensionValidation {
                    extension: extension.clone(),
                    error,
                }
            })?;
        }
        for usage in graph.resource_usages(&node_id) {
            match usage.resource {
                GraphResource::Buffer(handle) if !registry.contains_buffer(&handle) => {
                    return Err(RenderGraphValidationError::MissingUsageBuffer(handle));
                }
                GraphResource::Texture(handle) if !registry.contains_texture(&handle) => {
                    return Err(RenderGraphValidationError::MissingUsageTexture(handle));
                }
                _ => {}
            }
        }
        for command in node.commands() {
            if !registry.contains_pipeline(&command.pipeline) {
                return Err(RenderGraphValidationError::MissingPipeline(
                    command.pipeline,
                ));
            }
            for &(slot, bind_group, ref offsets) in &command.bind_groups {
                if bind_group_slot_index(slot, max_bind_groups).is_none() {
                    return Err(RenderGraphValidationError::InvalidBindGroupSlot {
                        slot,
                        max_slots: max_bind_groups,
                    });
                }
                if !registry.contains_bind_group(&bind_group) {
                    return Err(RenderGraphValidationError::MissingBindGroup(bind_group));
                }
                validate_bind_group_offsets(registry, bind_group, offsets)?;
                validate_render_pipeline_layout(registry, command.pipeline, slot, bind_group)?;
            }
            if let DrawAction::Indexed { mesh, .. } = command.action {
                if !registry.contains_mesh(&mesh) {
                    return Err(RenderGraphValidationError::MissingMesh(mesh));
                }
            }
            match command.action {
                DrawAction::Indirect { buffer, offset } => {
                    validate_indirect_buffer(registry, buffer, offset, 16)?;
                }
                DrawAction::IndexedIndirect {
                    mesh,
                    buffer,
                    offset,
                } => {
                    let Some((_, Some(_), _)) = registry.mesh(&mesh) else {
                        if !registry.contains_mesh(&mesh) {
                            return Err(RenderGraphValidationError::MissingMesh(mesh));
                        }
                        return Err(RenderGraphValidationError::MissingIndexBuffer(mesh));
                    };
                    validate_indirect_buffer(registry, buffer, offset, 20)?;
                }
                _ => {}
            }
        }
        for command in node.compute_commands() {
            if !registry.contains_compute_pipeline(&command.pipeline) {
                return Err(RenderGraphValidationError::MissingComputePipeline(
                    command.pipeline,
                ));
            }
            for &(slot, bind_group, ref offsets) in &command.bind_groups {
                if bind_group_slot_index(slot, max_bind_groups).is_none() {
                    return Err(RenderGraphValidationError::InvalidBindGroupSlot {
                        slot,
                        max_slots: max_bind_groups,
                    });
                }
                if !registry.contains_bind_group(&bind_group) {
                    return Err(RenderGraphValidationError::MissingBindGroup(bind_group));
                }
                validate_bind_group_offsets(registry, bind_group, offsets)?;
                validate_compute_pipeline_layout(registry, command.pipeline, slot, bind_group)?;
            }
            if let Some((buffer, offset)) = command.indirect {
                validate_indirect_buffer(registry, buffer, offset, 12)?;
            }
        }
        for command in node.copy_commands() {
            match command {
                CopyCommand::BufferToBuffer {
                    source,
                    destination,
                    source_offset,
                    destination_offset,
                    size,
                } => {
                    let Some(source_buffer) = registry.buffer(source) else {
                        return Err(RenderGraphValidationError::MissingBuffer(*source));
                    };
                    let Some(destination_buffer) = registry.buffer(destination) else {
                        return Err(RenderGraphValidationError::MissingBuffer(*destination));
                    };
                    if let Some(descriptor) = registry.buffer_descriptor(source) {
                        let required = wgpu::BufferUsages::COPY_SRC;
                        if !descriptor.usage.contains(required) {
                            return Err(RenderGraphValidationError::MissingBufferUsage {
                                handle: *source,
                                required_usage: required.bits(),
                                actual_usage: descriptor.usage.bits(),
                            });
                        }
                    }
                    if let Some(descriptor) = registry.buffer_descriptor(destination) {
                        let required = wgpu::BufferUsages::COPY_DST;
                        if !descriptor.usage.contains(required) {
                            return Err(RenderGraphValidationError::MissingBufferUsage {
                                handle: *destination,
                                required_usage: required.bits(),
                                actual_usage: descriptor.usage.bits(),
                            });
                        }
                    }
                    validate_copy_range(*source, *source_offset, *size, source_buffer.size())?;
                    validate_copy_range(
                        *destination,
                        *destination_offset,
                        *size,
                        destination_buffer.size(),
                    )?;
                }
                CopyCommand::TextureToTexture {
                    source,
                    destination,
                    source_mip_level,
                    destination_mip_level,
                    source_origin,
                    destination_origin,
                    extent,
                } => validate_texture_copy(
                    registry,
                    *source,
                    *destination,
                    *source_mip_level,
                    *destination_mip_level,
                    *source_origin,
                    *destination_origin,
                    *extent,
                    TextureAspect::All,
                )?,
                CopyCommand::TextureToTextureAspect {
                    source,
                    destination,
                    source_mip_level,
                    destination_mip_level,
                    source_origin,
                    destination_origin,
                    extent,
                    aspect,
                } => validate_texture_copy(
                    registry,
                    *source,
                    *destination,
                    *source_mip_level,
                    *destination_mip_level,
                    *source_origin,
                    *destination_origin,
                    *extent,
                    *aspect,
                )?,
            }
        }
        if let RenderNode::SubGraph { graph: child, .. } = node {
            validate_graph(
                registry,
                pool,
                child,
                max_bind_groups,
                extension_dispatchers,
            )?;
        }
    }
    Ok(())
}
