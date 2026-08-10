use thiserror::Error;
use wgpu::{Backends, DeviceDescriptor, Features, Instance, InstanceDescriptor, Limits, MemoryHints, PowerPreference, RequestAdapterOptions};
use crate::api::capabilities::GpuCapabilities;
use crate::api::engine::GpuEngine;

#[derive(Error, Debug)]
pub enum GpuError {
    #[error("No suitable GPU adapter found")]
    NoAdapterFound,
    #[error("Failed to request adapter: {0}")]
    AdapterRequestFailed(#[from] wgpu::RequestAdapterError),
    #[error("Failed to request device: {0}")]
    DeviceRequestFailed(#[from] wgpu::RequestDeviceError),
}

pub struct GpuEngineBuilder {
    backends: Backends,
    power_preference: PowerPreference,
    required_features: Features,
    required_limits: Limits,
}

impl Default for GpuEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuEngineBuilder {
    pub fn new() -> Self {
        Self {
            backends: Backends::all(),
            power_preference: PowerPreference::HighPerformance,
            required_features: Features::empty(),
            // Fallback limits that should work almost everywhere (WebGL2 compatible)
            required_limits: Limits::downlevel_webgl2_defaults(),
        }
    }

    pub fn with_backends(mut self, backends: Backends) -> Self {
        self.backends = backends;
        self
    }

    pub fn with_power_preference(mut self, pref: PowerPreference) -> Self {
        self.power_preference = pref;
        self
    }

    /// Khởi tạo GpuEngine. Quá trình này hoàn toàn không dính dáng đến Cửa sổ (Window/Surface).
    pub async fn build(self) -> Result<GpuEngine, GpuError> {
        log::info!("Initializing GPU Instance with backends: {:?}", self.backends);
        let instance = wgpu::Instance::default();

        log::info!("Requesting GPU Adapter...");
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: self.power_preference,
                compatible_surface: None, // Headless by default
                force_fallback_adapter: false,
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

        log::info!("Requesting Device & Queue...");
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("ifol_gpu_device"),
                    required_features: self.required_features,
                    required_limits: self.required_limits,
                    memory_hints: MemoryHints::Performance,
                    ..Default::default()
                }
            )
            .await?;

        Ok(GpuEngine::new(device, queue, capabilities))
    }
}
