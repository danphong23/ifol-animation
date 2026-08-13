use wgpu::{Features, Limits};

#[derive(Debug, Clone)]
pub struct GpuCapabilities {
    pub max_texture_dimension_2d: u32,
    pub max_bind_groups: u32,
    pub max_uniform_buffer_binding_size: u64,
    pub min_uniform_buffer_offset_alignment: u32,
    pub max_vertex_buffers: u32,
    pub max_vertex_attributes: u32,
    pub supports_compute: bool,
    /// Hỗ trợ giá trị `first_instance` khác 0 trong indirect draw.
    ///
    /// Indirect draw cơ bản là capability nền của WebGPU; feature này chỉ
    /// mô tả phần mở rộng cần kiểm tra riêng.
    pub supports_indirect_first_instance: bool,
    pub features: Features,
}

impl GpuCapabilities {
    pub fn new(limits: &Limits, features: &Features) -> Self {
        Self {
            max_texture_dimension_2d: limits.max_texture_dimension_2d,
            max_bind_groups: limits.max_bind_groups,
            max_uniform_buffer_binding_size: limits.max_uniform_buffer_binding_size,
            min_uniform_buffer_offset_alignment: limits.min_uniform_buffer_offset_alignment,
            max_vertex_buffers: limits.max_vertex_buffers,
            max_vertex_attributes: limits.max_vertex_attributes,
            supports_compute: limits.max_compute_workgroups_per_dimension > 0,
            supports_indirect_first_instance: features.contains(Features::INDIRECT_FIRST_INSTANCE),
            features: *features,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_snapshot_preserves_limits_and_features() {
        let mut limits = Limits::downlevel_webgl2_defaults();
        limits.max_texture_dimension_2d = 2048;
        let features = Features::INDIRECT_FIRST_INSTANCE;

        let capabilities = GpuCapabilities::new(&limits, &features);

        assert_eq!(capabilities.max_texture_dimension_2d, 2048);
        assert_eq!(
            capabilities.supports_compute,
            limits.max_compute_workgroups_per_dimension > 0
        );
        assert!(capabilities.supports_indirect_first_instance);
        assert!(capabilities.features.contains(Features::INDIRECT_FIRST_INSTANCE));
    }

    #[test]
    fn capability_snapshot_reports_absent_optional_features() {
        let limits = Limits::downlevel_webgl2_defaults();
        let capabilities = GpuCapabilities::new(&limits, &Features::empty());

        assert!(!capabilities.supports_indirect_first_instance);
        assert!(capabilities.features.is_empty());
    }
}
