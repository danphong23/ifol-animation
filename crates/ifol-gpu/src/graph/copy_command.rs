use crate::resources::handle::{BufferHandle, TextureHandle};

use crate::graph::TextureAspect;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyCommand {
    BufferToBuffer {
        source: BufferHandle,
        destination: BufferHandle,
        source_offset: u64,
        destination_offset: u64,
        size: u64,
    },
    TextureToTexture {
        source: TextureHandle,
        destination: TextureHandle,
        source_mip_level: u32,
        destination_mip_level: u32,
        source_origin: [u32; 3],
        destination_origin: [u32; 3],
        extent: [u32; 3],
    },
    TextureToTextureAspect {
        source: TextureHandle,
        destination: TextureHandle,
        source_mip_level: u32,
        destination_mip_level: u32,
        source_origin: [u32; 3],
        destination_origin: [u32; 3],
        extent: [u32; 3],
        aspect: TextureAspect,
    },
}

impl CopyCommand {
    pub fn buffer_to_buffer(source: BufferHandle, destination: BufferHandle, size: u64) -> Self {
        Self::BufferToBuffer {
            source,
            destination,
            source_offset: 0,
            destination_offset: 0,
            size,
        }
    }

    pub fn with_offsets(mut self, source_offset: u64, destination_offset: u64) -> Self {
        if let Self::BufferToBuffer {
            source_offset: source,
            destination_offset: destination,
            ..
        } = &mut self
        {
            *source = source_offset;
            *destination = destination_offset;
        }
        self
    }

    pub fn texture_to_texture(
        source: TextureHandle,
        destination: TextureHandle,
        extent: [u32; 3],
    ) -> Self {
        Self::TextureToTexture {
            source,
            destination,
            source_mip_level: 0,
            destination_mip_level: 0,
            source_origin: [0, 0, 0],
            destination_origin: [0, 0, 0],
            extent,
        }
    }

    pub fn texture_to_texture_aspect(
        source: TextureHandle,
        destination: TextureHandle,
        extent: [u32; 3],
        aspect: TextureAspect,
    ) -> Self {
        Self::TextureToTextureAspect {
            source,
            destination,
            source_mip_level: 0,
            destination_mip_level: 0,
            source_origin: [0, 0, 0],
            destination_origin: [0, 0, 0],
            extent,
            aspect,
        }
    }

    pub fn with_texture_mips(mut self, source_mip_level: u32, destination_mip_level: u32) -> Self {
        match &mut self {
            Self::TextureToTexture {
                source_mip_level: source,
                destination_mip_level: destination,
                ..
            }
            | Self::TextureToTextureAspect {
                source_mip_level: source,
                destination_mip_level: destination,
                ..
            } => {
                *source = source_mip_level;
                *destination = destination_mip_level;
            }
            _ => {}
        }
        self
    }
}
