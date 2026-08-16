use super::engine::GpuEngine;
use thiserror::Error;

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
}

pub struct ReadbackTicket {
    buffer: wgpu::Buffer,
    receiver: std::sync::mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
    submission: wgpu::SubmissionIndex,
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
    padded_bytes_per_row: u32,
}

impl ReadbackTicket {
    pub fn resolve_checked(
        self,
        device: &wgpu::Device,
    ) -> Result<(Vec<u8>, u32, u32), ReadbackError> {
        let _ = device.poll(wgpu::PollType::Wait {
            submission_index: Some(self.submission),
            timeout: None,
        });
        match self.receiver.recv() {
            Ok(Ok(())) => {}
            _ => return Err(ReadbackError::MapFailed),
        }
        let data = self
            .buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|_| ReadbackError::AccessFailed)?;
        let row_bytes = self
            .width
            .checked_mul(self.bytes_per_pixel)
            .ok_or(ReadbackError::ArithmeticOverflow)?;
        let capacity = row_bytes
            .checked_mul(self.height)
            .ok_or(ReadbackError::ArithmeticOverflow)? as usize;
        let mut pixels = Vec::with_capacity(capacity);
        for row in 0..self.height {
            let start = row
                .checked_mul(self.padded_bytes_per_row)
                .ok_or(ReadbackError::ArithmeticOverflow)? as usize;
            let end = start
                .checked_add(row_bytes as usize)
                .ok_or(ReadbackError::ArithmeticOverflow)?;
            let Some(row_data) = data.get(start..end) else {
                return Err(ReadbackError::AccessFailed);
            };
            pixels.extend_from_slice(row_data);
        }
        drop(data);
        self.buffer.unmap();
        Ok((pixels, self.width, self.height))
    }
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
        })
    }

    pub fn read_texture_to_bytes_with_format_checked(
        &self,
        texture: &wgpu::Texture,
        format: wgpu::TextureFormat,
    ) -> Result<(Vec<u8>, u32, u32), ReadbackError> {
        self.begin_texture_readback_checked(texture, format)?
            .resolve_checked(self.device())
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
mod tests {
    use super::{texture_format_bytes_per_pixel, ReadbackError};

    #[test]
    fn readback_format_width_is_explicit() {
        assert_eq!(
            texture_format_bytes_per_pixel(wgpu::TextureFormat::R8Unorm),
            Some(1)
        );
        assert_eq!(
            texture_format_bytes_per_pixel(wgpu::TextureFormat::Rgba8UnormSrgb),
            Some(4)
        );
        assert_eq!(
            texture_format_bytes_per_pixel(wgpu::TextureFormat::Rgba16Float),
            Some(8)
        );
        assert_eq!(
            texture_format_bytes_per_pixel(wgpu::TextureFormat::Depth32Float),
            None
        );
    }

    #[test]
    fn async_readback_ticket_resolves_after_submission() {
        let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("async-readback-test"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        engine.queue().write_texture(
            texture.as_image_copy(),
            &[1, 2, 3, 4],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            texture.size(),
        );

        let ticket = engine
            .begin_texture_readback_checked(&texture, wgpu::TextureFormat::Rgba8Unorm)
            .unwrap();
        let (pixels, width, height) = ticket.resolve_checked(engine.device()).unwrap();
        assert_eq!((width, height), (1, 1));
        assert_eq!(pixels, vec![1, 2, 3, 4]);
    }

    #[test]
    fn checked_readback_rejects_unsupported_format_with_typed_error() {
        let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("checked-readback-format-test"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        assert!(matches!(
            engine.begin_texture_readback_checked(&texture, wgpu::TextureFormat::Depth32Float),
            Err(ReadbackError::UnsupportedFormat(
                wgpu::TextureFormat::Depth32Float
            ))
        ));
    }
}
