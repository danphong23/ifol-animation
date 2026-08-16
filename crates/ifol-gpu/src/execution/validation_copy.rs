use crate::graph::{CopyCommand, RenderNode, TextureAspect};
use crate::resources::handle::BufferHandle;
use crate::resources::ResourceRegistry;

use super::validation::RenderGraphValidationError;
use super::validation_texture::validate_texture_copy;

pub(crate) fn validate_copy_commands(
    registry: &ResourceRegistry,
    node: &RenderNode,
) -> Result<(), RenderGraphValidationError> {
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
