use std::sync::Arc;
use crate::api::capabilities::GpuCapabilities;

pub struct GpuEngine {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    capabilities: GpuCapabilities,
}

impl GpuEngine {
    pub(crate) fn new(device: wgpu::Device, queue: wgpu::Queue, capabilities: GpuCapabilities) -> Self {
        Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            capabilities,
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
}
