pub use super::validation_errors::RenderGraphValidationError;
use crate::extensions::ExtensionDispatchRegistry;
use crate::graph::{
    CopyCommand, DrawAction, GraphResource, RenderGraph, RenderNode, RenderNodePool, RenderTarget,
    TextureAspect,
};
use crate::resources::handle::{
    BindGroupHandle, BufferHandle, ComputePipelineHandle, PipelineHandle, TextureHandle,
};
use crate::resources::registry::{ResourceRegistry, TextureResourceDescriptor};

pub(crate) fn bind_group_slot_index(slot: u32, max_slots: u32) -> Option<usize> {
    (slot < max_slots).then_some(slot as usize)
}

pub(crate) fn validate_bind_group_offsets(
    registry: &ResourceRegistry,
    handle: BindGroupHandle,
    offsets: &[u32],
) -> Result<(), RenderGraphValidationError> {
    let Some(descriptor) = registry.bind_group_descriptor(&handle) else {
        return Ok(());
    };
    if offsets.len() as u32 != descriptor.dynamic_offset_count {
        return Err(RenderGraphValidationError::InvalidDynamicOffsetCount {
            handle,
            expected: descriptor.dynamic_offset_count,
            actual: offsets.len() as u32,
        });
    }
    for &offset in offsets {
        if offset % descriptor.dynamic_offset_alignment != 0 {
            return Err(RenderGraphValidationError::InvalidDynamicOffsetAlignment {
                handle,
                offset,
                alignment: descriptor.dynamic_offset_alignment,
            });
        }
    }
    Ok(())
}

pub(crate) fn validate_render_pipeline_layout(
    registry: &ResourceRegistry,
    pipeline: PipelineHandle,
    slot: u32,
    bind_group: BindGroupHandle,
) -> Result<(), RenderGraphValidationError> {
    let Some(descriptor) = registry.pipeline_layout_descriptor(&pipeline) else {
        return Ok(());
    };
    let expected = descriptor
        .bind_group_layout_signatures
        .get(slot as usize)
        .copied()
        .flatten();
    let actual = registry
        .bind_group_descriptor(&bind_group)
        .map(|descriptor| descriptor.layout_signature);
    if expected.is_some() && actual.is_none() {
        return Err(RenderGraphValidationError::MissingPipelineLayoutMetadata {
            pipeline,
            bind_group,
            slot,
        });
    }
    if expected != actual {
        return Err(RenderGraphValidationError::PipelineLayoutMismatch {
            pipeline,
            slot,
            expected,
            actual,
        });
    }
    Ok(())
}

pub(crate) fn validate_compute_pipeline_layout(
    registry: &ResourceRegistry,
    pipeline: ComputePipelineHandle,
    slot: u32,
    bind_group: BindGroupHandle,
) -> Result<(), RenderGraphValidationError> {
    let Some(descriptor) = registry.compute_pipeline_layout_descriptor(&pipeline) else {
        return Ok(());
    };
    let expected = descriptor
        .bind_group_layout_signatures
        .get(slot as usize)
        .copied()
        .flatten();
    let actual = registry
        .bind_group_descriptor(&bind_group)
        .map(|descriptor| descriptor.layout_signature);
    if expected.is_some() && actual.is_none() {
        return Err(
            RenderGraphValidationError::MissingComputePipelineLayoutMetadata {
                pipeline,
                bind_group,
                slot,
            },
        );
    }
    if expected != actual {
        return Err(RenderGraphValidationError::ComputePipelineLayoutMismatch {
            pipeline,
            slot,
            expected,
            actual,
        });
    }
    Ok(())
}

pub(crate) fn format_has_stencil(format: wgpu::TextureFormat) -> bool {
    matches!(
        format,
        wgpu::TextureFormat::Stencil8
            | wgpu::TextureFormat::Depth24PlusStencil8
            | wgpu::TextureFormat::Depth32FloatStencil8
    )
}

pub(crate) fn validate_texture_copy(
    registry: &ResourceRegistry,
    source: TextureHandle,
    destination: TextureHandle,
    source_mip_level: u32,
    destination_mip_level: u32,
    source_origin: [u32; 3],
    destination_origin: [u32; 3],
    extent: [u32; 3],
    aspect: TextureAspect,
) -> Result<(), RenderGraphValidationError> {
    if !registry.contains_texture(&source) {
        return Err(RenderGraphValidationError::MissingTexture(source));
    }
    if !registry.contains_texture(&destination) {
        return Err(RenderGraphValidationError::MissingTexture(destination));
    }
    let Some(source_texture) = registry.owned_texture(&source) else {
        return Err(RenderGraphValidationError::MissingOwnedTexture(source));
    };
    let Some(destination_texture) = registry.owned_texture(&destination) else {
        return Err(RenderGraphValidationError::MissingOwnedTexture(destination));
    };
    let _ = (source_texture, destination_texture);
    let Some(source_descriptor) = registry.texture_descriptor(&source) else {
        return Err(RenderGraphValidationError::MissingTextureDescriptor(source));
    };
    let Some(destination_descriptor) = registry.texture_descriptor(&destination) else {
        return Err(RenderGraphValidationError::MissingTextureDescriptor(
            destination,
        ));
    };
    if source_descriptor.format != destination_descriptor.format {
        return Err(RenderGraphValidationError::TextureCopyFormatMismatch {
            source_handle: source,
            destination_handle: destination,
        });
    }
    if !texture_supports_aspect(source_descriptor.format, aspect) {
        return Err(RenderGraphValidationError::InvalidTextureAspect {
            handle: source,
            aspect,
        });
    }
    if !texture_supports_aspect(destination_descriptor.format, aspect) {
        return Err(RenderGraphValidationError::InvalidTextureAspect {
            handle: destination,
            aspect,
        });
    }
    let copy_src = wgpu::TextureUsages::COPY_SRC;
    let copy_dst = wgpu::TextureUsages::COPY_DST;
    if !source_descriptor.usage.contains(copy_src) {
        return Err(RenderGraphValidationError::MissingTextureUsage {
            handle: source,
            required_usage: copy_src.bits(),
            actual_usage: source_descriptor.usage.bits(),
        });
    }
    if !destination_descriptor.usage.contains(copy_dst) {
        return Err(RenderGraphValidationError::MissingTextureUsage {
            handle: destination,
            required_usage: copy_dst.bits(),
            actual_usage: destination_descriptor.usage.bits(),
        });
    }
    if extent.iter().any(|value| *value == 0) {
        return Err(RenderGraphValidationError::InvalidTextureCopyExtent { extent });
    }
    validate_texture_mip(
        source,
        source_mip_level,
        source_origin,
        extent,
        source_descriptor,
    )?;
    validate_texture_mip(
        destination,
        destination_mip_level,
        destination_origin,
        extent,
        destination_descriptor,
    )?;
    Ok(())
}

pub(crate) fn texture_supports_aspect(format: wgpu::TextureFormat, aspect: TextureAspect) -> bool {
    match aspect {
        TextureAspect::All => true,
        TextureAspect::DepthOnly => matches!(
            format,
            wgpu::TextureFormat::Depth16Unorm
                | wgpu::TextureFormat::Depth24Plus
                | wgpu::TextureFormat::Depth24PlusStencil8
                | wgpu::TextureFormat::Depth32Float
                | wgpu::TextureFormat::Depth32FloatStencil8
        ),
        TextureAspect::StencilOnly => matches!(
            format,
            wgpu::TextureFormat::Stencil8
                | wgpu::TextureFormat::Depth24PlusStencil8
                | wgpu::TextureFormat::Depth32FloatStencil8
        ),
    }
}

fn validate_texture_mip(
    handle: TextureHandle,
    mip_level: u32,
    origin: [u32; 3],
    extent: [u32; 3],
    descriptor: &TextureResourceDescriptor,
) -> Result<(), RenderGraphValidationError> {
    if mip_level >= descriptor.mip_level_count {
        return Err(RenderGraphValidationError::InvalidTextureMipLevel {
            handle,
            mip_level,
            mip_count: descriptor.mip_level_count,
        });
    }
    let mip_extent = [
        (descriptor.width >> mip_level).max(1),
        (descriptor.height >> mip_level).max(1),
        descriptor.depth_or_array_layers,
    ];
    let in_bounds =
        origin
            .iter()
            .zip(extent)
            .zip(mip_extent)
            .all(|((origin, extent), dimension)| {
                origin
                    .checked_add(extent)
                    .is_some_and(|end| end <= dimension)
            });
    if !in_bounds {
        return Err(RenderGraphValidationError::InvalidTextureCopyRange {
            handle,
            origin,
            extent,
            mip_extent,
        });
    }
    Ok(())
}

pub(crate) fn validate_copy_range(
    handle: BufferHandle,
    offset: u64,
    size: u64,
    buffer_size: u64,
) -> Result<(), RenderGraphValidationError> {
    let end = offset
        .checked_add(size)
        .ok_or(RenderGraphValidationError::InvalidCopyRange {
            handle,
            offset,
            size,
            buffer_size,
        })?;
    if end > buffer_size {
        return Err(RenderGraphValidationError::InvalidCopyRange {
            handle,
            offset,
            size,
            buffer_size,
        });
    }
    Ok(())
}

pub(crate) fn validate_indirect_buffer(
    registry: &ResourceRegistry,
    handle: BufferHandle,
    offset: u64,
    size: u64,
) -> Result<(), RenderGraphValidationError> {
    let Some(buffer) = registry.buffer(&handle) else {
        return Err(RenderGraphValidationError::MissingIndirectBuffer(handle));
    };
    if offset % 4 != 0
        || offset
            .checked_add(size)
            .is_none_or(|end| end > buffer.size())
    {
        return Err(RenderGraphValidationError::InvalidIndirectRange {
            handle,
            offset,
            size,
        });
    }
    if let Some(descriptor) = registry.buffer_descriptor(&handle) {
        let required = wgpu::BufferUsages::INDIRECT;
        if !descriptor.usage.contains(required) {
            return Err(RenderGraphValidationError::MissingIndirectBufferUsage {
                handle,
                required_usage: required.bits(),
                actual_usage: descriptor.usage.bits(),
            });
        }
    }
    Ok(())
}

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

    match graph.target {
        RenderTarget::Screen => {}
        RenderTarget::Offscreen {
            color,
            width,
            height,
        } => {
            if width == 0 || height == 0 {
                return Err(RenderGraphValidationError::InvalidTargetSize { width, height });
            }
            if !registry.contains_texture(&color) {
                return Err(RenderGraphValidationError::MissingTexture(color));
            }
            if let Some(descriptor) = registry.texture_descriptor(&color) {
                if descriptor.width != width || descriptor.height != height {
                    return Err(RenderGraphValidationError::TargetSizeMismatch {
                        handle: color,
                        width,
                        height,
                        actual_width: descriptor.width,
                        actual_height: descriptor.height,
                    });
                }
                let required = wgpu::TextureUsages::RENDER_ATTACHMENT;
                if !descriptor.usage.contains(required) {
                    return Err(RenderGraphValidationError::MissingTextureUsage {
                        handle: color,
                        required_usage: required.bits(),
                        actual_usage: descriptor.usage.bits(),
                    });
                }
                if descriptor.sample_count != 1 {
                    return Err(RenderGraphValidationError::UnsupportedSampleCount {
                        handle: color,
                        actual: descriptor.sample_count,
                    });
                }
            }
        }
        RenderTarget::OffscreenMsaa {
            color,
            resolve,
            width,
            height,
        } => {
            if width == 0 || height == 0 {
                return Err(RenderGraphValidationError::InvalidTargetSize { width, height });
            }
            let color_descriptor = registry
                .texture_descriptor(&color)
                .ok_or(RenderGraphValidationError::MissingTexture(color))?;
            if !registry.contains_texture(&color) {
                return Err(RenderGraphValidationError::MissingTexture(color));
            }
            if color_descriptor.width != width || color_descriptor.height != height {
                return Err(RenderGraphValidationError::TargetSizeMismatch {
                    handle: color,
                    width,
                    height,
                    actual_width: color_descriptor.width,
                    actual_height: color_descriptor.height,
                });
            }
            if color_descriptor.sample_count <= 1 {
                return Err(RenderGraphValidationError::UnsupportedSampleCount {
                    handle: color,
                    actual: color_descriptor.sample_count,
                });
            }
            if !color_descriptor
                .usage
                .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
            {
                return Err(RenderGraphValidationError::MissingTextureUsage {
                    handle: color,
                    required_usage: wgpu::TextureUsages::RENDER_ATTACHMENT.bits(),
                    actual_usage: color_descriptor.usage.bits(),
                });
            }
            let resolve_descriptor = registry
                .texture_descriptor(&resolve)
                .ok_or(RenderGraphValidationError::MissingResolveTexture(resolve))?;
            if resolve_descriptor.width != width || resolve_descriptor.height != height {
                return Err(RenderGraphValidationError::ResolveSizeMismatch {
                    color_width: width,
                    color_height: height,
                    resolve_width: resolve_descriptor.width,
                    resolve_height: resolve_descriptor.height,
                });
            }
            if resolve_descriptor.sample_count != 1 {
                return Err(RenderGraphValidationError::InvalidResolveSampleCount {
                    handle: resolve,
                    actual: resolve_descriptor.sample_count,
                });
            }
            if resolve_descriptor.format != color_descriptor.format {
                return Err(RenderGraphValidationError::ResolveFormatMismatch {
                    color: color_descriptor.format,
                    resolve: resolve_descriptor.format,
                });
            }
            if !resolve_descriptor
                .usage
                .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
            {
                return Err(RenderGraphValidationError::MissingTextureUsage {
                    handle: resolve,
                    required_usage: wgpu::TextureUsages::RENDER_ATTACHMENT.bits(),
                    actual_usage: resolve_descriptor.usage.bits(),
                });
            }
        }
    }

    let target_sample_count = match graph.target {
        RenderTarget::OffscreenMsaa { color, .. } => registry
            .texture_descriptor(&color)
            .map_or(1, |descriptor| descriptor.sample_count),
        _ => 1,
    };
    if let Some(depth) = graph.depth_stencil {
        if !registry.contains_texture(&depth) {
            return Err(RenderGraphValidationError::MissingTexture(depth));
        }
        if let Some(descriptor) = registry.texture_descriptor(&depth) {
            let required = wgpu::TextureUsages::RENDER_ATTACHMENT;
            if !descriptor.usage.contains(required) {
                return Err(RenderGraphValidationError::MissingTextureUsage {
                    handle: depth,
                    required_usage: required.bits(),
                    actual_usage: descriptor.usage.bits(),
                });
            }
            if descriptor.sample_count != target_sample_count {
                return Err(RenderGraphValidationError::DepthSampleCountMismatch {
                    handle: depth,
                    expected: target_sample_count,
                    actual: descriptor.sample_count,
                });
            }
        }
    }

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
