use crate::graph::{CopyCommand, TextureAspect};
use crate::resources::handle::TextureHandle;
use crate::resources::registry::ResourceRegistry;

use super::validation::RenderGraphValidationError;

pub(crate) fn encode_copy_command(
    encoder: &mut wgpu::CommandEncoder,
    registry: &ResourceRegistry,
    command: &CopyCommand,
) -> Result<(), RenderGraphValidationError> {
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
            encoder.copy_buffer_to_buffer(
                source_buffer,
                *source_offset,
                destination_buffer,
                *destination_offset,
                *size,
            );
        }
        CopyCommand::TextureToTexture {
            source,
            destination,
            source_mip_level,
            destination_mip_level,
            source_origin,
            destination_origin,
            extent,
        } => encode_texture_copy(
            encoder,
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
        } => encode_texture_copy(
            encoder,
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
    Ok(())
}

fn encode_texture_copy(
    encoder: &mut wgpu::CommandEncoder,
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
    let Some(source_texture) = registry.owned_texture(&source) else {
        return Err(RenderGraphValidationError::MissingOwnedTexture(source));
    };
    let Some(destination_texture) = registry.owned_texture(&destination) else {
        return Err(RenderGraphValidationError::MissingOwnedTexture(destination));
    };
    let origin = |value: [u32; 3]| wgpu::Origin3d {
        x: value[0],
        y: value[1],
        z: value[2],
    };
    let extent = wgpu::Extent3d {
        width: extent[0],
        height: extent[1],
        depth_or_array_layers: extent[2],
    };
    encoder.copy_texture_to_texture(
        wgpu::TexelCopyTextureInfo {
            texture: source_texture,
            mip_level: source_mip_level,
            origin: origin(source_origin),
            aspect: to_wgpu_texture_aspect(aspect),
        },
        wgpu::TexelCopyTextureInfo {
            texture: destination_texture,
            mip_level: destination_mip_level,
            origin: origin(destination_origin),
            aspect: to_wgpu_texture_aspect(aspect),
        },
        extent,
    );
    Ok(())
}

fn to_wgpu_texture_aspect(aspect: TextureAspect) -> wgpu::TextureAspect {
    match aspect {
        TextureAspect::All => wgpu::TextureAspect::All,
        TextureAspect::DepthOnly => wgpu::TextureAspect::DepthOnly,
        TextureAspect::StencilOnly => wgpu::TextureAspect::StencilOnly,
    }
}
