use super::builder::{GpuEngineBuilder, GpuError};
use super::capabilities::GpuCapabilities;
use wgpu::{DeviceDescriptor, RequestAdapterOptions};

impl<'a> GpuEngineBuilder<'a> {
    /// Khởi tạo GpuEngine. Quá trình này hoàn toàn không dính dáng đến Cửa sổ (Window/Surface).
    pub async fn build(self) -> Result<super::engine::GpuEngine<'a>, GpuError> {
        log::info!(
            "Initializing GPU Instance with backends: {:?}",
            self.backends
        );

        log::info!("Requesting GPU Adapter...");
        let adapter = self
            .instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: self.power_preference,
                compatible_surface: self.surface.as_ref(),
                force_fallback_adapter: self.force_fallback_adapter,
                apply_limit_buckets: false,
            })
            .await?;

        let adapter_info = adapter.get_info();
        log::info!(
            "Picked Adapter: {} ({:?}) - Backend: {:?}",
            adapter_info.name,
            adapter_info.device_type,
            adapter_info.backend
        );

        let capabilities = GpuCapabilities::new(&adapter.limits(), &adapter.features());
        log::info!("Hardware Capabilities: {:?}", capabilities);
        capabilities.validate_requirements(self.required_features, &self.required_limits)?;

        log::info!("Requesting Device & Queue...");
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("ifol_gpu_device"),
                required_features: self.required_features,
                required_limits: self.required_limits,
                memory_hints: wgpu::MemoryHints::Performance,
                ..Default::default()
            })
            .await?;

        let mut surface_config = None;
        if let Some(surface) = &self.surface {
            let Some(config) = surface.get_default_config(&adapter, 1, 1) else {
                return Err(GpuError::SurfaceUnsupported);
            };
            surface.configure(&device, &config);
            surface_config = Some(config);
        }

        Ok(super::engine::GpuEngine::new(
            device,
            queue,
            adapter_info,
            capabilities,
            self.surface,
            surface_config,
        ))
    }
}
