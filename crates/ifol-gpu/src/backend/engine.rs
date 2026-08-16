use std::sync::Arc;
use thiserror::Error;
use crate::backend::capabilities::GpuCapabilities;
pub use super::readback::{ReadbackError, ReadbackTicket};

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

#[derive(Debug, Error)]
pub enum TextureSaveError {
    #[error(transparent)]
    Readback(#[from] ReadbackError),
    #[error("could not create parent directory {path:?}: {source}")]
    CreateDirectory { path: std::path::PathBuf, source: std::io::Error },
    #[error("image encoding failed: {0}")]
    Encode(#[from] image::ImageError),
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

#[cfg(test)]
mod tests {
    use super::{SurfaceResizeError, TextureSaveError};

    #[test]
    fn headless_surface_lifecycle_returns_typed_errors() {
        let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
        assert_eq!(engine.try_resize_surface(0, 8), Err(SurfaceResizeError::InvalidSize));
        assert_eq!(engine.try_resize_surface(8, 8), Err(SurfaceResizeError::Unavailable));
        assert_eq!(engine.reconfigure_surface(), Err(SurfaceResizeError::Unavailable));
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
