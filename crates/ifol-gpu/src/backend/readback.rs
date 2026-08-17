use super::engine::GpuEngine;
use crate::resources::{ResourceRegistry, TextureHandle};
use thiserror::Error;

#[path = "readback_ticket.rs"]
mod readback_ticket;
pub use readback_ticket::ReadbackTicket;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReadbackError {
    #[error("texture dimensions must be non-zero")]
    InvalidExtent,
    #[error("texture format {0:?} is not supported by core readback")]
    UnsupportedFormat(wgpu::TextureFormat),
    #[error("readback layout arithmetic overflowed")]
    ArithmeticOverflow,
    #[error("GPU readback buffer mapping failed")]
    MapFailed,
    #[error("GPU readback buffer could not be accessed")]
    AccessFailed,
    #[error("texture handle {0:?} is not an owned texture with a readback descriptor")]
    ResourceUnavailable(TextureHandle),
}

/// Raw texture bytes together with the dimensions and format contract used
/// for the copy. The bytes are unpadded row data.
#[derive(Debug, PartialEq, Eq)]
pub struct RawTextureReadback {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

impl<'a> GpuEngine<'a> {
    pub fn begin_texture_readback_checked(
        &self,
        texture: &wgpu::Texture,
        format: wgpu::TextureFormat,
    ) -> Result<ReadbackTicket, ReadbackError> {
        let width = texture.size().width;
        let height = texture.size().height;
        if width == 0 || height == 0 {
            return Err(ReadbackError::InvalidExtent);
        }
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let bytes_per_pixel = texture_format_bytes_per_pixel(format)
            .ok_or(ReadbackError::UnsupportedFormat(format))?;
        let unpadded_bytes = width
            .checked_mul(bytes_per_pixel)
            .ok_or(ReadbackError::ArithmeticOverflow)?;
        let padded_bytes = unpadded_bytes
            .checked_add(align - 1)
            .ok_or(ReadbackError::ArithmeticOverflow)?
            & !(align - 1);
        let buffer_size = padded_bytes
            .checked_mul(height)
            .ok_or(ReadbackError::ArithmeticOverflow)? as u64;

        let buffer = self.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("ReadbackBuffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes),
                    rows_per_image: Some(height),
                },
            },
            texture.size(),
        );
        let submission_index = self.queue().submit(std::iter::once(encoder.finish()));

        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |v| {
            let _ = tx.send(v);
        });

        Ok(ReadbackTicket {
            buffer,
            receiver: rx,
            submission: submission_index,
            width,
            height,
            bytes_per_pixel,
            padded_bytes_per_row: padded_bytes,
            format,
        })
    }

    pub fn read_texture_to_raw_with_format_checked(
        &self,
        texture: &wgpu::Texture,
        format: wgpu::TextureFormat,
    ) -> Result<RawTextureReadback, ReadbackError> {
        self.begin_texture_readback_checked(texture, format)?
            .resolve_checked(self.device())
    }

    /// Reads an owned registry texture using the format stored with its
    /// descriptor. View-only registrations intentionally remain on the
    /// explicit texture/format API because they do not retain a copy source.
    pub fn read_texture_to_raw_from_registry_checked(
        &self,
        registry: &ResourceRegistry,
        handle: &TextureHandle,
    ) -> Result<RawTextureReadback, ReadbackError> {
        let texture = registry
            .owned_texture(handle)
            .ok_or(ReadbackError::ResourceUnavailable(*handle))?;
        let format = registry
            .texture_descriptor(handle)
            .map(|descriptor| descriptor.format)
            .ok_or(ReadbackError::ResourceUnavailable(*handle))?;
        self.read_texture_to_raw_with_format_checked(texture, format)
    }

}

fn texture_format_bytes_per_pixel(format: wgpu::TextureFormat) -> Option<u32> {
    match format {
        wgpu::TextureFormat::R8Unorm
        | wgpu::TextureFormat::R8Snorm
        | wgpu::TextureFormat::R8Uint
        | wgpu::TextureFormat::R8Sint => Some(1),
        wgpu::TextureFormat::R16Uint
        | wgpu::TextureFormat::R16Sint
        | wgpu::TextureFormat::R16Float => Some(2),
        wgpu::TextureFormat::Rg8Unorm
        | wgpu::TextureFormat::Rg8Snorm
        | wgpu::TextureFormat::Rg8Uint
        | wgpu::TextureFormat::Rg8Sint
        | wgpu::TextureFormat::R32Uint
        | wgpu::TextureFormat::R32Sint
        | wgpu::TextureFormat::R32Float
        | wgpu::TextureFormat::Rgba8Unorm
        | wgpu::TextureFormat::Rgba8UnormSrgb
        | wgpu::TextureFormat::Rgba8Snorm
        | wgpu::TextureFormat::Rgba8Uint
        | wgpu::TextureFormat::Rgba8Sint
        | wgpu::TextureFormat::Bgra8Unorm
        | wgpu::TextureFormat::Bgra8UnormSrgb => Some(4),
        wgpu::TextureFormat::Rg16Uint
        | wgpu::TextureFormat::Rg16Sint
        | wgpu::TextureFormat::Rg16Float
        | wgpu::TextureFormat::Rgba16Uint
        | wgpu::TextureFormat::Rgba16Sint
        | wgpu::TextureFormat::Rgba16Float
        | wgpu::TextureFormat::Rg32Uint
        | wgpu::TextureFormat::Rg32Sint
        | wgpu::TextureFormat::Rg32Float => Some(8),
        wgpu::TextureFormat::Rgba32Uint
        | wgpu::TextureFormat::Rgba32Sint
        | wgpu::TextureFormat::Rgba32Float => Some(16),
        _ => None,
    }
}

#[cfg(test)]
#[path = "readback_tests.rs"]
mod tests;
