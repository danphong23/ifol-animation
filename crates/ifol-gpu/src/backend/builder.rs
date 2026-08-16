use thiserror::Error;
use wgpu::{Backends, Features, InstanceDescriptor, Limits, PowerPreference};

#[derive(Error, Debug)]
pub enum GpuError {
    #[error("No suitable GPU adapter found")]
    NoAdapterFound,
    #[error("Failed to request adapter: {0}")]
    AdapterRequestFailed(#[from] wgpu::RequestAdapterError),
    #[error("Failed to request device: {0}")]
    DeviceRequestFailed(#[from] wgpu::RequestDeviceError),
    #[error("GPU adapter does not satisfy requested capabilities: {0}")]
    InsufficientCapabilities(#[from] crate::backend::capabilities::CapabilityError),
    #[error("selected adapter cannot configure the provided surface")]
    SurfaceUnsupported,
}

pub struct GpuEngineBuilder<'a> {
    pub(super) instance: wgpu::Instance,
    pub(super) backends: Backends,
    pub(super) power_preference: PowerPreference,
    pub(super) force_fallback_adapter: bool,
    pub(super) required_features: Features,
    pub(super) required_limits: Limits,
    pub(super) surface: Option<wgpu::Surface<'a>>,
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
            force_fallback_adapter: false,
            required_features: Features::empty(),
            required_limits: Limits::downlevel_webgl2_defaults(),
            surface: None,
        }
    }

    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    pub fn backends(&self) -> wgpu::Backends {
        self.backends
    }

    pub fn required_features(&self) -> wgpu::Features {
        self.required_features
    }

    pub fn required_limits(&self) -> &wgpu::Limits {
        &self.required_limits
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

    pub fn force_fallback_adapter(&self) -> bool {
        self.force_fallback_adapter
    }

    pub fn with_force_fallback_adapter(mut self, force: bool) -> Self {
        self.force_fallback_adapter = force;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_keeps_backend_and_requirement_policy_explicit() {
        let limits = Limits::downlevel_webgl2_defaults();
        let builder = GpuEngineBuilder::new()
            .with_backends(Backends::VULKAN | Backends::GL)
            .with_required_features(Features::INDIRECT_FIRST_INSTANCE)
            .with_required_limits(limits.clone());
        assert_eq!(builder.backends(), Backends::VULKAN | Backends::GL);
        assert_eq!(
            builder.required_features(),
            Features::INDIRECT_FIRST_INSTANCE
        );
        assert_eq!(builder.required_limits(), &limits);
        assert!(!builder.force_fallback_adapter());
        assert!(builder
            .with_force_fallback_adapter(true)
            .force_fallback_adapter());
    }

    #[test]
    fn builder_backend_policy_reaches_runtime_adapter_request() {
        let result = pollster::block_on(
            GpuEngineBuilder::new()
                .with_backends(Backends::VULKAN)
                .build(),
        );
        match result {
            Ok(engine) => {
                assert!(engine.capabilities().max_bind_groups > 0);
                assert_eq!(engine.adapter_info().backend, wgpu::Backend::Vulkan);
            }
            Err(GpuError::NoAdapterFound | GpuError::AdapterRequestFailed(_)) => {}
            Err(error) => panic!("unexpected Vulkan backend setup error: {error}"),
        }
    }

    #[test]
    fn builder_gl_policy_is_an_explicit_optional_backend_probe() {
        let result =
            pollster::block_on(GpuEngineBuilder::new().with_backends(Backends::GL).build());
        match result {
            Ok(engine) => assert!(engine.capabilities().max_bind_groups > 0),
            Err(GpuError::NoAdapterFound | GpuError::AdapterRequestFailed(_)) => {}
            Err(error) => panic!("unexpected GL backend setup error: {error}"),
        }
    }

    #[test]
    fn builder_dx12_policy_is_an_explicit_optional_backend_probe() {
        let result = pollster::block_on(
            GpuEngineBuilder::new()
                .with_backends(Backends::DX12)
                .build(),
        );
        match result {
            Ok(engine) => assert!(engine.capabilities().max_bind_groups > 0),
            Err(GpuError::NoAdapterFound | GpuError::AdapterRequestFailed(_)) => {}
            Err(error) => panic!("unexpected DX12 backend setup error: {error}"),
        }
    }

    #[test]
    fn builder_fallback_adapter_policy_is_runtime_selectable() {
        let result = pollster::block_on(
            GpuEngineBuilder::new()
                .with_force_fallback_adapter(true)
                .build(),
        );
        match result {
            Ok(engine) => assert!(engine.capabilities().max_bind_groups > 0),
            Err(GpuError::NoAdapterFound | GpuError::AdapterRequestFailed(_)) => {}
            Err(error) => panic!("unexpected fallback adapter setup error: {error}"),
        }
    }
}
