use std::sync::Arc;
use thiserror::Error;
use crate::backend::capabilities::GpuCapabilities;

pub struct GpuEngine<'a> {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    adapter_info: wgpu::AdapterInfo,
    capabilities: GpuCapabilities,
    surface: Option<wgpu::Surface<'a>>,
    surface_config: std::sync::RwLock<Option<wgpu::SurfaceConfiguration>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceResizeError {
    InvalidSize,
    Unavailable,
    LockPoisoned,
}

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

#[derive(Debug, Error)]
pub enum TextureSaveError {
    #[error(transparent)]
    Readback(#[from] ReadbackError),
    #[error("could not create parent directory {path:?}: {source}")]
    CreateDirectory { path: std::path::PathBuf, source: std::io::Error },
    #[error("image encoding failed: {0}")]
    Encode(#[from] image::ImageError),
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
    pub fn resolve(self, device: &wgpu::Device) -> Result<(Vec<u8>, u32, u32), &'static str> {
        self.resolve_checked(device).map_err(|error| match error {
            ReadbackError::InvalidExtent => "Invalid texture extent for readback",
            ReadbackError::UnsupportedFormat(_) => "Unsupported texture format for readback",
            ReadbackError::ArithmeticOverflow => "Readback layout arithmetic overflowed",
            ReadbackError::MapFailed => "Failed to map buffer for readback",
            ReadbackError::AccessFailed => "Failed to access mapped readback buffer",
        })
    }

    pub fn resolve_checked(self, device: &wgpu::Device) -> Result<(Vec<u8>, u32, u32), ReadbackError> {
        let _ = device.poll(wgpu::PollType::Wait { submission_index: Some(self.submission), timeout: None });
        match self.receiver.recv() {
            Ok(Ok(())) => {}
            _ => return Err(ReadbackError::MapFailed),
        }
        let data = self
            .buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|_| ReadbackError::AccessFailed)?;
        let row_bytes = self.width.checked_mul(self.bytes_per_pixel).ok_or(ReadbackError::ArithmeticOverflow)?;
        let capacity = row_bytes.checked_mul(self.height).ok_or(ReadbackError::ArithmeticOverflow)? as usize;
        let mut pixels = Vec::with_capacity(capacity);
        for row in 0..self.height {
            let start = row.checked_mul(self.padded_bytes_per_row).ok_or(ReadbackError::ArithmeticOverflow)? as usize;
            let end = start.checked_add(row_bytes as usize).ok_or(ReadbackError::ArithmeticOverflow)?;
            let Some(row_data) = data.get(start..end) else { return Err(ReadbackError::AccessFailed); };
            pixels.extend_from_slice(row_data);
        }
        drop(data);
        self.buffer.unmap();
        Ok((pixels, self.width, self.height))
    }
}

impl<'a> GpuEngine<'a> {
    pub(crate) fn new(
        device: wgpu::Device, 
        queue: wgpu::Queue, 
        adapter_info: wgpu::AdapterInfo,
        capabilities: GpuCapabilities,
        surface: Option<wgpu::Surface<'a>>,
        surface_config: Option<wgpu::SurfaceConfiguration>,
    ) -> Self {
        Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info,
            capabilities,
            surface,
            surface_config: std::sync::RwLock::new(surface_config),
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn capabilities(&self) -> &GpuCapabilities {
        &self.capabilities
    }

    /// Thông tin adapter thực tế đã tạo device, dùng cho diagnostics và
    /// capability/runtime matrix của host.
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    pub fn surface(&self) -> Option<&wgpu::Surface<'a>> {
        self.surface.as_ref()
    }

    pub fn try_resize_surface(&self, width: u32, height: u32) -> Result<(), SurfaceResizeError> {
        if width == 0 || height == 0 {
            return Err(SurfaceResizeError::InvalidSize);
        }
        let surface = self.surface.as_ref().ok_or(SurfaceResizeError::Unavailable)?;
        let mut config_lock = self.surface_config.write().map_err(|_| SurfaceResizeError::LockPoisoned)?;
        let config = config_lock.as_mut().ok_or(SurfaceResizeError::Unavailable)?;
        config.width = width;
        config.height = height;
        surface.configure(&self.device, config);
        Ok(())
    }

    pub fn reconfigure_surface(&self) -> Result<(), SurfaceResizeError> {
        let surface = self.surface.as_ref().ok_or(SurfaceResizeError::Unavailable)?;
        let config_lock = self.surface_config.read().map_err(|_| SurfaceResizeError::LockPoisoned)?;
        let config = config_lock.as_ref().ok_or(SurfaceResizeError::Unavailable)?;
        surface.configure(&self.device, config);
        Ok(())
    }

    pub fn surface_format(&self) -> Option<wgpu::TextureFormat> {
        self.surface_config.read().ok().and_then(|config| config.as_ref().map(|c| c.format))
    }

    /// Đọc toàn bộ byte của một Texture (2D) từ VRAM về CPU. Dùng để xuất file ảnh (PNG/JPEG) 
    /// phục vụ Automated Snapshot Testing hoặc kết xuất video (Offline Rendering).
    /// API legacy giả định `Rgba8UnormSrgb`.
    pub fn read_texture_to_bytes(&self, texture: &wgpu::Texture) -> Result<(Vec<u8>, u32, u32), &'static str> {
        self.read_texture_to_bytes_with_format(texture, wgpu::TextureFormat::Rgba8UnormSrgb)
    }

    /// Readback theo format thật của texture. Core không tự đoán channel count
    /// từ texture handle vì `wgpu::Texture` không expose descriptor sau khi tạo.
    pub fn begin_texture_readback(
        &self,
        texture: &wgpu::Texture,
        format: wgpu::TextureFormat,
    ) -> Result<ReadbackTicket, &'static str> {
        self.begin_texture_readback_checked(texture, format).map_err(|error| match error {
            ReadbackError::InvalidExtent => "Invalid texture extent for readback",
            ReadbackError::UnsupportedFormat(_) => "Unsupported texture format for readback",
            ReadbackError::ArithmeticOverflow => "Readback layout arithmetic overflowed",
            ReadbackError::MapFailed | ReadbackError::AccessFailed => "Failed to initialize readback",
        })
    }

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
        let bytes_per_pixel = texture_format_bytes_per_pixel(format).ok_or(ReadbackError::UnsupportedFormat(format))?;
        let unpadded_bytes = width.checked_mul(bytes_per_pixel).ok_or(ReadbackError::ArithmeticOverflow)?;
        let padded_bytes = unpadded_bytes.checked_add(align - 1).ok_or(ReadbackError::ArithmeticOverflow)? & !(align - 1);
        let buffer_size = padded_bytes.checked_mul(height).ok_or(ReadbackError::ArithmeticOverflow)? as u64;
        
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ReadbackBuffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo { 
                buffer: &buffer, 
                layout: wgpu::TexelCopyBufferLayout { 
                    offset: 0, 
                    bytes_per_row: Some(padded_bytes), 
                    rows_per_image: Some(height) 
                } 
            },
            texture.size()
        );
        let submission_index = self.queue.submit(std::iter::once(encoder.finish()));
        
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

    pub fn read_texture_to_bytes_with_format(
        &self,
        texture: &wgpu::Texture,
        format: wgpu::TextureFormat,
    ) -> Result<(Vec<u8>, u32, u32), &'static str> {
        self.read_texture_to_bytes_with_format_checked(texture, format).map_err(|error| match error {
            ReadbackError::InvalidExtent => "Invalid texture extent for readback",
            ReadbackError::UnsupportedFormat(_) => "Unsupported texture format for readback",
            ReadbackError::ArithmeticOverflow => "Readback layout arithmetic overflowed",
            ReadbackError::MapFailed => "Failed to map buffer for readback",
            ReadbackError::AccessFailed => "Failed to access mapped readback buffer",
        })
    }

    pub fn read_texture_to_bytes_with_format_checked(
        &self,
        texture: &wgpu::Texture,
        format: wgpu::TextureFormat,
    ) -> Result<(Vec<u8>, u32, u32), ReadbackError> {
        self.begin_texture_readback_checked(texture, format)?.resolve_checked(&self.device)
    }

    /// Đọc kết quả từ VRAM và lưu trực tiếp ra file ảnh trên ổ cứng.
    /// Bạn có thể truyền đường dẫn và tên file tùy ý. Đuôi file (.png, .jpg) sẽ tự định dạng loại ảnh.
    pub fn save_texture_to_file<P: AsRef<std::path::Path>>(&self, texture: &wgpu::Texture, path: P) -> Result<(), &'static str> {
        let (pixels, width, height) = self.read_texture_to_bytes(texture)?;
        
        // Đảm bảo thư mục tồn tại trước khi lưu
        if let Some(parent) = path.as_ref().parent() {
            if !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        
        if image::save_buffer(path, &pixels, width, height, image::ColorType::Rgba8).is_err() {
            return Err("Failed to save image to disk");
        }
        Ok(())
    }

    pub fn save_texture_to_file_checked<P: AsRef<std::path::Path>>(
        &self,
        texture: &wgpu::Texture,
        path: P,
    ) -> Result<(), TextureSaveError> {
        let (pixels, width, height) = self
            .read_texture_to_bytes_with_format_checked(texture, wgpu::TextureFormat::Rgba8UnormSrgb)?;
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|source| TextureSaveError::CreateDirectory {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
        }
        image::save_buffer(path, &pixels, width, height, image::ColorType::Rgba8)?;
        Ok(())
    }
}

fn texture_format_bytes_per_pixel(format: wgpu::TextureFormat) -> Option<u32> {
    match format {
        wgpu::TextureFormat::R8Unorm | wgpu::TextureFormat::R8Snorm |
        wgpu::TextureFormat::R8Uint | wgpu::TextureFormat::R8Sint => Some(1),
        wgpu::TextureFormat::R16Uint | wgpu::TextureFormat::R16Sint |
        wgpu::TextureFormat::R16Float => Some(2),
        wgpu::TextureFormat::Rg8Unorm | wgpu::TextureFormat::Rg8Snorm |
        wgpu::TextureFormat::Rg8Uint | wgpu::TextureFormat::Rg8Sint |
        wgpu::TextureFormat::R32Uint | wgpu::TextureFormat::R32Sint |
        wgpu::TextureFormat::R32Float | wgpu::TextureFormat::Rgba8Unorm |
        wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Rgba8Snorm |
        wgpu::TextureFormat::Rgba8Uint | wgpu::TextureFormat::Rgba8Sint |
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => Some(4),
        wgpu::TextureFormat::Rg16Uint | wgpu::TextureFormat::Rg16Sint |
        wgpu::TextureFormat::Rg16Float | wgpu::TextureFormat::Rgba16Uint |
        wgpu::TextureFormat::Rgba16Sint | wgpu::TextureFormat::Rgba16Float |
        wgpu::TextureFormat::Rg32Uint | wgpu::TextureFormat::Rg32Sint |
        wgpu::TextureFormat::Rg32Float => Some(8),
        wgpu::TextureFormat::Rgba32Uint | wgpu::TextureFormat::Rgba32Sint |
        wgpu::TextureFormat::Rgba32Float => Some(16),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{texture_format_bytes_per_pixel, ReadbackError, SurfaceResizeError, TextureSaveError};

    #[test]
    fn readback_format_width_is_explicit() {
        assert_eq!(texture_format_bytes_per_pixel(wgpu::TextureFormat::R8Unorm), Some(1));
        assert_eq!(texture_format_bytes_per_pixel(wgpu::TextureFormat::Rgba8UnormSrgb), Some(4));
        assert_eq!(texture_format_bytes_per_pixel(wgpu::TextureFormat::Rgba16Float), Some(8));
        assert_eq!(texture_format_bytes_per_pixel(wgpu::TextureFormat::Depth32Float), None);
    }

    #[test]
    fn headless_surface_lifecycle_returns_typed_errors() {
        let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
        assert_eq!(engine.try_resize_surface(0, 8), Err(SurfaceResizeError::InvalidSize));
        assert_eq!(engine.try_resize_surface(8, 8), Err(SurfaceResizeError::Unavailable));
        assert_eq!(engine.reconfigure_surface(), Err(SurfaceResizeError::Unavailable));
    }

    #[test]
    fn async_readback_ticket_resolves_after_submission() {
        let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("async-readback-test"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
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
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
            texture.size(),
        );

        let ticket = engine.begin_texture_readback(&texture, wgpu::TextureFormat::Rgba8Unorm).unwrap();
        let (pixels, width, height) = ticket.resolve(engine.device()).unwrap();
        assert_eq!((width, height), (1, 1));
        assert_eq!(pixels, vec![1, 2, 3, 4]);
    }

    #[test]
    fn checked_readback_rejects_unsupported_format_with_typed_error() {
        let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("checked-readback-format-test"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        assert!(matches!(
            engine.begin_texture_readback_checked(&texture, wgpu::TextureFormat::Depth32Float),
            Err(ReadbackError::UnsupportedFormat(wgpu::TextureFormat::Depth32Float))
        ));
    }

    #[test]
    fn checked_texture_save_reports_encode_failure() {
        let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("checked-save-error-test"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let parent = std::env::temp_dir().join(format!("ifol_gpu_save_parent_{}", std::process::id()));
        std::fs::write(&parent, b"file, not directory").unwrap();
        let result = engine.save_texture_to_file_checked(&texture, parent.join("output.png"));
        let _ = std::fs::remove_file(&parent);
        assert!(matches!(result, Err(TextureSaveError::Encode(_))));
    }
}
