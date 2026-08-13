use std::collections::HashMap;
use thiserror::Error;
use crate::render::handle::{BindGroupHandle, MeshHandle, PipelineHandle, TextureHandle};

type ResourceVersion = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureResourceDescriptor {
    pub width: u32,
    pub height: u32,
    pub depth_or_array_layers: u32,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
    pub mip_level_count: u32,
    pub sample_count: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ResourceDescriptorError {
    #[error("texture width and height must be non-zero, got {width}x{height}")]
    InvalidExtent { width: u32, height: u32 },
    #[error("texture layer count must be non-zero")]
    InvalidLayerCount,
    #[error("texture mip level count must be non-zero")]
    InvalidMipCount,
    #[error("texture sample count must be non-zero")]
    InvalidSampleCount,
    #[error("texture usage must not be empty")]
    EmptyUsage,
    #[error("texture extent {width}x{height} exceeds device limit {max_dimension}")]
    ExceedsDimensionLimit { width: u32, height: u32, max_dimension: u32 },
}

impl TextureResourceDescriptor {
    pub fn validate(&self, max_dimension: u32) -> Result<(), ResourceDescriptorError> {
        if self.width == 0 || self.height == 0 {
            return Err(ResourceDescriptorError::InvalidExtent { width: self.width, height: self.height });
        }
        if self.depth_or_array_layers == 0 {
            return Err(ResourceDescriptorError::InvalidLayerCount);
        }
        if self.mip_level_count == 0 {
            return Err(ResourceDescriptorError::InvalidMipCount);
        }
        if self.sample_count == 0 {
            return Err(ResourceDescriptorError::InvalidSampleCount);
        }
        if self.usage.is_empty() {
            return Err(ResourceDescriptorError::EmptyUsage);
        }
        if self.width > max_dimension || self.height > max_dimension {
            return Err(ResourceDescriptorError::ExceedsDimensionLimit {
                width: self.width,
                height: self.height,
                max_dimension,
            });
        }
        Ok(())
    }
}

#[derive(Default)]
struct ResourceVersions {
    textures: HashMap<TextureHandle, ResourceVersion>,
    pipelines: HashMap<PipelineHandle, ResourceVersion>,
    meshes: HashMap<MeshHandle, ResourceVersion>,
    bind_groups: HashMap<BindGroupHandle, ResourceVersion>,
}

/// Nơi ánh xạ từ Handle siêu nhẹ (u64) sang các đối tượng nặng của GPU (Buffer, Texture, Pipeline)
#[derive(Default)]
pub struct ResourceRegistry {
    pub textures: HashMap<TextureHandle, (wgpu::TextureView, wgpu::TextureFormat)>,
    pub pipelines: HashMap<PipelineHandle, wgpu::RenderPipeline>,
    /// Lưu trữ Mesh: (VBO, Option<(IBO, IndexFormat)>, Số lượng Index/Vertex mặc định)
    pub meshes: HashMap<MeshHandle, (wgpu::Buffer, Option<(wgpu::Buffer, wgpu::IndexFormat)>, u32)>, 
    pub bind_groups: HashMap<BindGroupHandle, wgpu::BindGroup>,
    texture_descriptors: HashMap<TextureHandle, TextureResourceDescriptor>,
    versions: ResourceVersions,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Đăng ký hoặc thay thế texture và tăng version để compiled artifact biết
    /// rằng resource backing đã thay đổi.
    pub fn insert_texture(
        &mut self,
        handle: TextureHandle,
        texture: (wgpu::TextureView, wgpu::TextureFormat),
    ) -> Option<(wgpu::TextureView, wgpu::TextureFormat)> {
        let old = self.textures.insert(handle, texture);
        self.bump_texture_version(handle);
        old
    }

    pub fn texture(&self, handle: &TextureHandle) -> Option<&(wgpu::TextureView, wgpu::TextureFormat)> {
        self.textures.get(handle)
    }

    pub fn insert_texture_with_descriptor(
        &mut self,
        handle: TextureHandle,
        texture: wgpu::TextureView,
        descriptor: TextureResourceDescriptor,
        max_dimension: u32,
    ) -> Result<Option<(wgpu::TextureView, wgpu::TextureFormat)>, ResourceDescriptorError> {
        descriptor.validate(max_dimension)?;
        let old = self.textures.insert(handle, (texture, descriptor.format));
        self.texture_descriptors.insert(handle, descriptor);
        self.bump_texture_version(handle);
        Ok(old)
    }

    pub fn texture_descriptor(&self, handle: &TextureHandle) -> Option<&TextureResourceDescriptor> {
        self.texture_descriptors.get(handle)
    }

    pub fn texture_version(&self, handle: &TextureHandle) -> ResourceVersion {
        self.versions.textures.get(handle).copied().unwrap_or(0)
    }

    pub fn mark_texture_changed(&mut self, handle: TextureHandle) {
        self.bump_texture_version(handle);
    }

    pub fn insert_pipeline(
        &mut self,
        handle: PipelineHandle,
        pipeline: wgpu::RenderPipeline,
    ) -> Option<wgpu::RenderPipeline> {
        let old = self.pipelines.insert(handle, pipeline);
        self.bump_pipeline_version(handle);
        old
    }

    pub fn pipeline(&self, handle: &PipelineHandle) -> Option<&wgpu::RenderPipeline> {
        self.pipelines.get(handle)
    }

    pub fn pipeline_version(&self, handle: &PipelineHandle) -> ResourceVersion {
        self.versions.pipelines.get(handle).copied().unwrap_or(0)
    }

    pub fn mark_pipeline_changed(&mut self, handle: PipelineHandle) {
        self.bump_pipeline_version(handle);
    }

    pub fn mesh_version(&self, handle: &MeshHandle) -> ResourceVersion {
        self.versions.meshes.get(handle).copied().unwrap_or(0)
    }

    pub fn mark_mesh_changed(&mut self, handle: MeshHandle) {
        Self::bump_version(&mut self.versions.meshes, handle);
    }

    pub fn bind_group_version(&self, handle: &BindGroupHandle) -> ResourceVersion {
        self.versions.bind_groups.get(handle).copied().unwrap_or(0)
    }

    pub fn mark_bind_group_changed(&mut self, handle: BindGroupHandle) {
        Self::bump_version(&mut self.versions.bind_groups, handle);
    }

    pub fn remove_texture(&mut self, handle: &TextureHandle) -> Option<(wgpu::TextureView, wgpu::TextureFormat)> {
        let old = self.textures.remove(handle);
        self.texture_descriptors.remove(handle);
        if old.is_some() {
            self.bump_texture_version(*handle);
        }
        old
    }

    pub fn remove_pipeline(&mut self, handle: &PipelineHandle) -> Option<wgpu::RenderPipeline> {
        let old = self.pipelines.remove(handle);
        if old.is_some() {
            self.bump_pipeline_version(*handle);
        }
        old
    }

    pub fn remove_mesh(&mut self, handle: &MeshHandle) -> Option<(wgpu::Buffer, Option<(wgpu::Buffer, wgpu::IndexFormat)>, u32)> {
        let old = self.meshes.remove(handle);
        if old.is_some() {
            Self::bump_version(&mut self.versions.meshes, *handle);
        }
        old
    }

    pub fn remove_bind_group(&mut self, handle: &BindGroupHandle) -> Option<wgpu::BindGroup> {
        let old = self.bind_groups.remove(handle);
        if old.is_some() {
            Self::bump_version(&mut self.versions.bind_groups, *handle);
        }
        old
    }

    fn bump_texture_version(&mut self, handle: TextureHandle) {
        Self::bump_version(&mut self.versions.textures, handle);
    }

    fn bump_pipeline_version(&mut self, handle: PipelineHandle) {
        Self::bump_version(&mut self.versions.pipelines, handle);
    }

    fn bump_version<H: Copy + Eq + std::hash::Hash>(
        versions: &mut HashMap<H, ResourceVersion>,
        handle: H,
    ) {
        let version = versions.entry(handle).or_insert(0);
        *version = version.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_version_starts_at_zero_and_marks_changes() {
        let mut registry = ResourceRegistry::new();
        let handle = TextureHandle(11);

        assert_eq!(registry.texture_version(&handle), 0);
        registry.mark_texture_changed(handle);
        assert_eq!(registry.texture_version(&handle), 1);
        registry.mark_texture_changed(handle);
        assert_eq!(registry.texture_version(&handle), 2);
    }

    #[test]
    fn versions_are_typed_and_independent() {
        let mut registry = ResourceRegistry::new();
        registry.mark_texture_changed(TextureHandle(1));
        registry.mark_pipeline_changed(PipelineHandle(1));

        assert_eq!(registry.texture_version(&TextureHandle(1)), 1);
        assert_eq!(registry.pipeline_version(&PipelineHandle(1)), 1);
        assert_eq!(registry.texture_version(&TextureHandle(2)), 0);
        assert_eq!(registry.pipeline_version(&PipelineHandle(2)), 0);
    }

    fn valid_descriptor() -> TextureResourceDescriptor {
        TextureResourceDescriptor {
            width: 128,
            height: 64,
            depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            mip_level_count: 1,
            sample_count: 1,
        }
    }

    #[test]
    fn texture_descriptor_accepts_valid_input() {
        assert_eq!(valid_descriptor().validate(1024), Ok(()));
    }

    #[test]
    fn texture_descriptor_rejects_invalid_extent_and_limit() {
        let mut descriptor = valid_descriptor();
        descriptor.width = 0;
        assert_eq!(
            descriptor.validate(1024),
            Err(ResourceDescriptorError::InvalidExtent { width: 0, height: 64 })
        );

        descriptor = valid_descriptor();
        descriptor.width = 2048;
        assert_eq!(
            descriptor.validate(1024),
            Err(ResourceDescriptorError::ExceedsDimensionLimit { width: 2048, height: 64, max_dimension: 1024 })
        );
    }

    #[test]
    fn texture_descriptor_rejects_missing_shape_and_usage_fields() {
        let mut descriptor = valid_descriptor();
        descriptor.mip_level_count = 0;
        assert_eq!(descriptor.validate(1024), Err(ResourceDescriptorError::InvalidMipCount));
        descriptor = valid_descriptor();
        descriptor.usage = wgpu::TextureUsages::empty();
        assert_eq!(descriptor.validate(1024), Err(ResourceDescriptorError::EmptyUsage));
    }
}
