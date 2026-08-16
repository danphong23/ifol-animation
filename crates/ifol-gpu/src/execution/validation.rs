use thiserror::Error;

use crate::graph::TextureAspect;
use crate::resources::handle::{
    BindGroupHandle, BufferHandle, ComputePipelineHandle, MeshHandle, PipelineHandle, RenderNodeId,
    TextureHandle,
};
use crate::resources::registry::{ResourceRegistry, TextureResourceDescriptor};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RenderGraphValidationError {
    #[error("extension operation {0:?} has no executor dispatch registered")]
    UnsupportedExtension(crate::extensions::ExtensionId),
    #[error("extension operation {extension:?} failed validation: {error}")]
    ExtensionValidation {
        extension: crate::extensions::ExtensionId,
        error: crate::extensions::ExtensionValidationError,
    },
    #[error("extension operation {extension:?} failed during dispatch: {error}")]
    ExtensionDispatch {
        extension: crate::extensions::ExtensionId,
        error: crate::extensions::ExtensionExecutionError,
    },
    #[error("render node {0:?} does not exist in the node pool")]
    MissingNode(RenderNodeId),
    #[error("render graph dependency cycle involves node {0:?}")]
    DependencyCycle(RenderNodeId),
    #[error("render graph dependency references node {0:?} outside the graph")]
    DependencyOutsideGraph(RenderNodeId),
    #[error("texture resource {0:?} is missing")]
    MissingTexture(TextureHandle),
    #[error("pipeline resource {0:?} is missing")]
    MissingPipeline(PipelineHandle),
    #[error("compute pipeline resource {0:?} is missing")]
    MissingComputePipeline(ComputePipelineHandle),
    #[error("buffer resource {0:?} is missing")]
    MissingBuffer(BufferHandle),
    #[error("declared resource usage references missing buffer {0:?}")]
    MissingUsageBuffer(BufferHandle),
    #[error("declared resource usage references missing texture {0:?}")]
    MissingUsageTexture(TextureHandle),
    #[error("buffer {handle:?} is missing required usage bits {required_usage:#x}; actual {actual_usage:#x}")]
    MissingBufferUsage {
        handle: BufferHandle,
        required_usage: u32,
        actual_usage: u32,
    },
    #[error("owned texture resource {0:?} is required for texture copy")]
    MissingOwnedTexture(TextureHandle),
    #[error("texture resource {0:?} has no descriptor metadata")]
    MissingTextureDescriptor(TextureHandle),
    #[error(
        "texture copy formats differ: source {source_handle:?}, destination {destination_handle:?}"
    )]
    TextureCopyFormatMismatch {
        source_handle: TextureHandle,
        destination_handle: TextureHandle,
    },
    #[error("texture copy extent must be non-zero, got {extent:?}")]
    InvalidTextureCopyExtent { extent: [u32; 3] },
    #[error("texture {handle:?} does not support copy aspect {aspect:?}")]
    InvalidTextureAspect {
        handle: TextureHandle,
        aspect: TextureAspect,
    },
    #[error(
        "texture copy mip level {mip_level} is invalid for {handle:?} (mip count {mip_count})"
    )]
    InvalidTextureMipLevel {
        handle: TextureHandle,
        mip_level: u32,
        mip_count: u32,
    },
    #[error("texture copy range for {handle:?} exceeds mip extent {mip_extent:?}: origin {origin:?}, extent {extent:?}")]
    InvalidTextureCopyRange {
        handle: TextureHandle,
        origin: [u32; 3],
        extent: [u32; 3],
        mip_extent: [u32; 3],
    },
    #[error("copy range for buffer {handle:?} exceeds buffer size: offset {offset}, size {size}, buffer size {buffer_size}")]
    InvalidCopyRange {
        handle: BufferHandle,
        offset: u64,
        size: u64,
        buffer_size: u64,
    },
    #[error("mesh resource {0:?} is missing")]
    MissingMesh(MeshHandle),
    #[error("bind group resource {0:?} is missing")]
    MissingBindGroup(BindGroupHandle),
    #[error("indirect buffer {0:?} is missing")]
    MissingIndirectBuffer(BufferHandle),
    #[error("indirect buffer {handle:?} is missing required usage bits {required_usage:#x}; actual {actual_usage:#x}")]
    MissingIndirectBufferUsage {
        handle: BufferHandle,
        required_usage: u32,
        actual_usage: u32,
    },
    #[error("indirect buffer {handle:?} range is invalid: offset {offset}, size {size}")]
    InvalidIndirectRange {
        handle: BufferHandle,
        offset: u64,
        size: u64,
    },
    #[error("indexed indirect draw requires mesh {0:?} to have an index buffer")]
    MissingIndexBuffer(MeshHandle),
    #[error("bind group slot {slot} is outside the device limit {max_slots}")]
    InvalidBindGroupSlot { slot: u32, max_slots: u32 },
    #[error("bind group {handle:?} expects {expected} dynamic offsets, got {actual}")]
    InvalidDynamicOffsetCount {
        handle: BindGroupHandle,
        expected: u32,
        actual: u32,
    },
    #[error("dynamic offset {offset} for bind group {handle:?} is not aligned to {alignment}")]
    InvalidDynamicOffsetAlignment {
        handle: BindGroupHandle,
        offset: u32,
        alignment: u32,
    },
    #[error("pipeline {pipeline:?} has no bind-group layout metadata for bind group {bind_group:?} at slot {slot}")]
    MissingPipelineLayoutMetadata {
        pipeline: PipelineHandle,
        bind_group: BindGroupHandle,
        slot: u32,
    },
    #[error("compute pipeline {pipeline:?} has no bind-group layout metadata for bind group {bind_group:?} at slot {slot}")]
    MissingComputePipelineLayoutMetadata {
        pipeline: ComputePipelineHandle,
        bind_group: BindGroupHandle,
        slot: u32,
    },
    #[error("pipeline {pipeline:?} layout mismatch at slot {slot}: expected {expected:?}, actual {actual:?}")]
    PipelineLayoutMismatch {
        pipeline: PipelineHandle,
        slot: u32,
        expected: Option<u64>,
        actual: Option<u64>,
    },
    #[error("compute pipeline {pipeline:?} layout mismatch at slot {slot}: expected {expected:?}, actual {actual:?}")]
    ComputePipelineLayoutMismatch {
        pipeline: ComputePipelineHandle,
        slot: u32,
        expected: Option<u64>,
        actual: Option<u64>,
    },
    #[error("render target dimensions must be non-zero, got {width}x{height}")]
    InvalidTargetSize { width: u32, height: u32 },
    #[error("texture {handle:?} has descriptor size {actual_width}x{actual_height}, graph requested {width}x{height}")]
    TargetSizeMismatch {
        handle: TextureHandle,
        width: u32,
        height: u32,
        actual_width: u32,
        actual_height: u32,
    },
    #[error("texture {handle:?} is missing required usage bits {required_usage:#x}; actual {actual_usage:#x}")]
    MissingTextureUsage {
        handle: TextureHandle,
        required_usage: u32,
        actual_usage: u32,
    },
    #[error("texture {handle:?} uses sample count {actual}, but this render path supports only sample count 1")]
    UnsupportedSampleCount { handle: TextureHandle, actual: u32 },
    #[error("MSAA resolve texture {0:?} is missing")]
    MissingResolveTexture(TextureHandle),
    #[error("MSAA resolve texture {handle:?} must be single-sample, got {actual}")]
    InvalidResolveSampleCount { handle: TextureHandle, actual: u32 },
    #[error("MSAA color and resolve formats differ: color {color:?}, resolve {resolve:?}")]
    ResolveFormatMismatch {
        color: wgpu::TextureFormat,
        resolve: wgpu::TextureFormat,
    },
    #[error("MSAA color and resolve dimensions differ: color {color_width}x{color_height}, resolve {resolve_width}x{resolve_height}")]
    ResolveSizeMismatch {
        color_width: u32,
        color_height: u32,
        resolve_width: u32,
        resolve_height: u32,
    },
    #[error("depth texture {handle:?} sample count mismatch: expected {expected}, got {actual}")]
    DepthSampleCountMismatch {
        handle: TextureHandle,
        expected: u32,
        actual: u32,
    },
}

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
