use wgpu::{Features, Limits};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CapabilityError {
    #[error("required GPU features are unavailable: requested {requested:?}, available {available:?}")]
    MissingFeatures { requested: Features, available: Features },
    #[error("required GPU limits are unavailable")]
    InsufficientLimits,
}

#[derive(Debug, Clone)]
pub struct GpuCapabilities {
    pub limits: Limits,
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
    /// Có thể dùng timestamp query cho profiler GPU tùy chọn.
    pub supports_timestamp_queries: bool,
    pub features: Features,
}

impl GpuCapabilities {
    pub fn new(limits: &Limits, features: &Features) -> Self {
        Self {
            limits: limits.clone(),
            max_texture_dimension_2d: limits.max_texture_dimension_2d,
            max_bind_groups: limits.max_bind_groups,
            max_uniform_buffer_binding_size: limits.max_uniform_buffer_binding_size,
            min_uniform_buffer_offset_alignment: limits.min_uniform_buffer_offset_alignment,
            max_vertex_buffers: limits.max_vertex_buffers,
            max_vertex_attributes: limits.max_vertex_attributes,
            supports_compute: limits.max_compute_workgroups_per_dimension > 0,
            supports_indirect_first_instance: features.contains(Features::INDIRECT_FIRST_INSTANCE),
            supports_timestamp_queries: features.contains(Features::TIMESTAMP_QUERY),
            features: *features,
        }
    }

    pub fn validate_requirements(&self, required_features: Features, required_limits: &Limits) -> Result<(), CapabilityError> {
        if !self.features.contains(required_features) {
            return Err(CapabilityError::MissingFeatures { requested: required_features, available: self.features });
        }
        if !required_limits.check_limits(&self.limits) {
            return Err(CapabilityError::InsufficientLimits);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_snapshot_preserves_limits_and_features() {
        let mut limits = Limits::downlevel_webgl2_defaults();
        limits.max_texture_dimension_2d = 2048;
        let features = Features::INDIRECT_FIRST_INSTANCE | Features::TIMESTAMP_QUERY;

        let capabilities = GpuCapabilities::new(&limits, &features);

        assert_eq!(capabilities.max_texture_dimension_2d, 2048);
        assert_eq!(
            capabilities.supports_compute,
            limits.max_compute_workgroups_per_dimension > 0
        );
        assert!(capabilities.supports_indirect_first_instance);
        assert!(capabilities.supports_timestamp_queries);
        assert!(capabilities.features.contains(Features::INDIRECT_FIRST_INSTANCE));
    }

    #[test]
    fn capability_snapshot_reports_absent_optional_features() {
        let limits = Limits::downlevel_webgl2_defaults();
        let capabilities = GpuCapabilities::new(&limits, &Features::empty());

        assert!(!capabilities.supports_indirect_first_instance);
        assert!(!capabilities.supports_timestamp_queries);
        assert!(capabilities.features.is_empty());
    }

    #[test]
    fn requirements_validation_reports_feature_and_limit_mismatch() {
        let limits = Limits::downlevel_webgl2_defaults();
        let capabilities = GpuCapabilities::new(&limits, &Features::empty());
        assert_eq!(
            capabilities.validate_requirements(Features::INDIRECT_FIRST_INSTANCE, &limits),
            Err(CapabilityError::MissingFeatures { requested: Features::INDIRECT_FIRST_INSTANCE, available: Features::empty() })
        );
        let mut required_limits = limits.clone();
        required_limits.max_texture_dimension_2d = limits.max_texture_dimension_2d + 1;
        assert_eq!(capabilities.validate_requirements(Features::empty(), &required_limits), Err(CapabilityError::InsufficientLimits));
    }

    #[test]
    fn requirements_validation_accepts_subset_of_snapshot() {
        let limits = Limits::downlevel_webgl2_defaults();
        let capabilities = GpuCapabilities::new(&limits, &Features::INDIRECT_FIRST_INSTANCE);
        assert_eq!(capabilities.validate_requirements(Features::empty(), &limits), Ok(()));
    }
}
