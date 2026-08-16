use crate::graph::TextureAspect;
use crate::resources::handle::{BufferHandle, TextureHandle};
use crate::resources::{ResourceRegistry, TextureResourceDescriptor};

use super::validation::RenderGraphValidationError;

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
