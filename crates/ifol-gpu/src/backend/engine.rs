use std::sync::Arc;
use crate::backend::capabilities::GpuCapabilities;
pub use super::readback::{RawTextureReadback, ReadbackError, ReadbackTicket};
pub use super::texture_save::TextureSaveError;

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

}

#[cfg(test)]
mod tests {
    use super::SurfaceResizeError;

    #[test]
    fn headless_surface_lifecycle_returns_typed_errors() {
        let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
        assert_eq!(engine.try_resize_surface(0, 8), Err(SurfaceResizeError::InvalidSize));
        assert_eq!(engine.try_resize_surface(8, 8), Err(SurfaceResizeError::Unavailable));
        assert_eq!(engine.reconfigure_surface(), Err(SurfaceResizeError::Unavailable));
    }

}
