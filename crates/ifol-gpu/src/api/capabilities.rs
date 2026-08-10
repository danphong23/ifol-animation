use wgpu::{Features, Limits};

#[derive(Debug, Clone)]
pub struct GpuCapabilities {
    pub max_texture_dimension_2d: u32,
    pub max_bind_groups: u32,
    pub max_uniform_buffer_binding_size: u64,
    pub max_vertex_buffers: u32,
    pub max_vertex_attributes: u32,
    pub supports_compute: bool,
}

impl GpuCapabilities {
    pub fn new(limits: &Limits, _features: &Features) -> Self {
        Self {
            max_texture_dimension_2d: limits.max_texture_dimension_2d,
            max_bind_groups: limits.max_bind_groups,
            max_uniform_buffer_binding_size: limits.max_uniform_buffer_binding_size,
            max_vertex_buffers: limits.max_vertex_buffers,
            max_vertex_attributes: limits.max_vertex_attributes,
            // Compute is supported if max_compute_workgroups_per_dimension is > 0
            supports_compute: limits.max_compute_workgroups_per_dimension > 0,
        }
    }
}
