use thiserror::Error;
use wgpu::{Backends, DeviceDescriptor, Features, InstanceDescriptor, Limits, MemoryHints, PowerPreference, RequestAdapterOptions};
use crate::api::capabilities::GpuCapabilities;


#[derive(Error, Debug)]
pub enum GpuError {
    #[error("No suitable GPU adapter found")]
    NoAdapterFound,
    #[error("Failed to request adapter: {0}")]
    AdapterRequestFailed(#[from] wgpu::RequestAdapterError),
    #[error("Failed to request device: {0}")]
    DeviceRequestFailed(#[from] wgpu::RequestDeviceError),
}

pub struct GpuEngineBuilder<'a> {
    instance: wgpu::Instance,
    backends: Backends,
    power_preference: PowerPreference,
    required_features: Features,
    required_limits: Limits,
    surface: Option<wgpu::Surface<'a>>,
}

impl<'a> Default for GpuEngineBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> GpuEngineBuilder<'a> {
    pub fn new() -> Self {
        let backends = Backends::all();
        Self {
            instance: wgpu::Instance::new(InstanceDescriptor {
                backends,
                ..InstanceDescriptor::new_without_display_handle()
            }),
            backends,
            power_preference: PowerPreference::HighPerformance,
            required_features: Features::empty(),
            required_limits: Limits::downlevel_webgl2_defaults(),
            surface: None,
        }
    }

    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    pub fn with_surface(mut self, surface: wgpu::Surface<'a>) -> Self {
        self.surface = Some(surface);
        self
    }

    pub fn with_backends(mut self, backends: Backends) -> Self {
        self.backends = backends;
        self.instance = wgpu::Instance::new(InstanceDescriptor {
            backends,
            ..InstanceDescriptor::new_without_display_handle()
        });
        self
    }

    pub fn with_power_preference(mut self, pref: PowerPreference) -> Self {
        self.power_preference = pref;
        self
    }

    pub fn with_required_features(mut self, features: Features) -> Self {
        self.required_features = features;
        self
    }

    pub fn with_required_limits(mut self, limits: Limits) -> Self {
        self.required_limits = limits;
        self
    }

    /// Khởi tạo GpuEngine. Quá trình này hoàn toàn không dính dáng đến Cửa sổ (Window/Surface).
    pub async fn build(self) -> Result<crate::api::engine::GpuEngine<'a>, GpuError> {
        log::info!("Initializing GPU Instance with backends: {:?}", self.backends);

        log::info!("Requesting GPU Adapter...");
        let adapter = self.instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: self.power_preference,
                compatible_surface: self.surface.as_ref(),
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

        let mut surface_config = None;
        if let Some(surface) = &self.surface {
            if let Some(config) = surface.get_default_config(&adapter, 1, 1) {
                surface.configure(&device, &config);
                surface_config = Some(config);
            }
        }

        Ok(crate::api::engine::GpuEngine::new(device, queue, capabilities, self.surface, surface_config))
    }
}
