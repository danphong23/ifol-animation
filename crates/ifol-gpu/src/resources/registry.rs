use std::collections::HashMap;
use thiserror::Error;
use crate::memory::{DeferredDestructionQueue, SubmissionId};
use crate::resources::handle::{BindGroupHandle, BufferHandle, ComputePipelineHandle, MeshHandle, PipelineHandle, TextureHandle};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferResourceDescriptor {
    pub size: u64,
    pub usage: wgpu::BufferUsages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshResourceDescriptor {
    pub vertex_buffer_size: u64,
    pub vertex_count: u32,
    pub index_buffer_size: Option<u64>,
    pub index_format: Option<wgpu::IndexFormat>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MeshDescriptorError {
    #[error("mesh vertex buffer size must be non-zero")]
    InvalidVertexBufferSize,
    #[error("mesh vertex count must be non-zero")]
    InvalidVertexCount,
    #[error("mesh index buffer size must be non-zero when present")]
    InvalidIndexBufferSize,
    #[error("mesh index format requires an index buffer")]
    IndexFormatWithoutBuffer,
}

impl MeshResourceDescriptor {
    pub fn validate(&self) -> Result<(), MeshDescriptorError> {
        if self.vertex_buffer_size == 0 {
            return Err(MeshDescriptorError::InvalidVertexBufferSize);
        }
        if self.vertex_count == 0 {
            return Err(MeshDescriptorError::InvalidVertexCount);
        }
        if self.index_buffer_size == Some(0) {
            return Err(MeshDescriptorError::InvalidIndexBufferSize);
        }
        if self.index_format.is_some() && self.index_buffer_size.is_none() {
            return Err(MeshDescriptorError::IndexFormatWithoutBuffer);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindGroupResourceDescriptor {
    pub dynamic_offset_count: u32,
    pub dynamic_offset_alignment: u32,
    pub layout_signature: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineLayoutResourceDescriptor {
    pub bind_group_layout_signatures: Vec<Option<u64>>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BindGroupDescriptorError {
    #[error("dynamic offset alignment must be zero when there are no dynamic offsets")]
    UnexpectedAlignmentWithoutOffsets,
    #[error("dynamic offset alignment must be a non-zero power of two")]
    InvalidAlignment,
}

impl BindGroupResourceDescriptor {
    pub fn validate(&self) -> Result<(), BindGroupDescriptorError> {
        if self.dynamic_offset_count == 0 {
            return if self.dynamic_offset_alignment == 0 {
                Ok(())
            } else {
                Err(BindGroupDescriptorError::UnexpectedAlignmentWithoutOffsets)
            };
        }
        if self.dynamic_offset_alignment == 0 || !self.dynamic_offset_alignment.is_power_of_two() {
            return Err(BindGroupDescriptorError::InvalidAlignment);
        }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BufferDescriptorError {
    #[error("buffer size must be non-zero")]
    InvalidSize,
    #[error("buffer usage must not be empty")]
    EmptyUsage,
}

impl BufferResourceDescriptor {
    pub fn validate(&self) -> Result<(), BufferDescriptorError> {
        if self.size == 0 { return Err(BufferDescriptorError::InvalidSize); }
        if self.usage.is_empty() { return Err(BufferDescriptorError::EmptyUsage); }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ResourceDescriptorError {
    #[error("texture width and height must be non-zero, got {width}x{height}")]
    InvalidExtent { width: u32, height: u32 },
    #[error("texture layer count must be non-zero")]
    InvalidLayerCount,
    #[error("texture mip level count must be non-zero")]
    InvalidMipCount,
    #[error("texture mip level count {mip_level_count} exceeds maximum {max_mip_level_count} for extent {width}x{height}")]
    MipCountExceedsExtent { mip_level_count: u32, max_mip_level_count: u32, width: u32, height: u32 },
    #[error("texture sample count must be non-zero")]
    InvalidSampleCount,
    #[error("texture sample count {sample_count} must be a power of two")]
    InvalidSampleCountValue { sample_count: u32 },
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
        let max_mip_level_count = u32::BITS - self.width.max(self.height).leading_zeros();
        if self.mip_level_count > max_mip_level_count {
            return Err(ResourceDescriptorError::MipCountExceedsExtent {
                mip_level_count: self.mip_level_count,
                max_mip_level_count,
                width: self.width,
                height: self.height,
            });
        }
        if self.sample_count == 0 {
            return Err(ResourceDescriptorError::InvalidSampleCount);
        }
        if !self.sample_count.is_power_of_two() {
            return Err(ResourceDescriptorError::InvalidSampleCountValue { sample_count: self.sample_count });
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

pub struct OwnedTextureResource {
    texture: wgpu::Texture,
    descriptor: TextureResourceDescriptor,
}

impl OwnedTextureResource {
    pub fn texture(&self) -> &wgpu::Texture { &self.texture }
    pub fn descriptor(&self) -> TextureResourceDescriptor { self.descriptor }
}

#[derive(Default)]
struct ResourceVersions {
    textures: HashMap<TextureHandle, ResourceVersion>,
    pipelines: HashMap<PipelineHandle, ResourceVersion>,
    compute_pipelines: HashMap<ComputePipelineHandle, ResourceVersion>,
    buffers: HashMap<BufferHandle, ResourceVersion>,
    meshes: HashMap<MeshHandle, ResourceVersion>,
    bind_groups: HashMap<BindGroupHandle, ResourceVersion>,
}

/// Nơi ánh xạ từ Handle siêu nhẹ (u64) sang các đối tượng nặng của GPU (Buffer, Texture, Pipeline)
#[derive(Default)]
pub struct ResourceRegistry {
    textures: HashMap<TextureHandle, (wgpu::TextureView, wgpu::TextureFormat)>,
    pipelines: HashMap<PipelineHandle, wgpu::RenderPipeline>,
    compute_pipelines: HashMap<ComputePipelineHandle, wgpu::ComputePipeline>,
    pipeline_layout_descriptors: HashMap<PipelineHandle, PipelineLayoutResourceDescriptor>,
    compute_pipeline_layout_descriptors: HashMap<ComputePipelineHandle, PipelineLayoutResourceDescriptor>,
    buffers: HashMap<BufferHandle, wgpu::Buffer>,
    /// Lưu trữ Mesh: (VBO, Option<(IBO, IndexFormat)>, Số lượng Index/Vertex mặc định)
    meshes: HashMap<MeshHandle, (wgpu::Buffer, Option<(wgpu::Buffer, wgpu::IndexFormat)>, u32)>,
    mesh_descriptors: HashMap<MeshHandle, MeshResourceDescriptor>,
    bind_groups: HashMap<BindGroupHandle, wgpu::BindGroup>,
    bind_group_descriptors: HashMap<BindGroupHandle, BindGroupResourceDescriptor>,
    buffer_descriptors: HashMap<BufferHandle, BufferResourceDescriptor>,
    texture_descriptors: HashMap<TextureHandle, TextureResourceDescriptor>,
    owned_textures: HashMap<TextureHandle, OwnedTextureResource>,
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
        self.owned_textures.remove(&handle);
        // Compatibility insert không có descriptor mới; không giữ metadata cũ
        // để tránh validate graph dựa trên texture đã bị thay thế.
        self.texture_descriptors.remove(&handle);
        self.bump_texture_version(handle);
        old
    }

    pub fn texture(&self, handle: &TextureHandle) -> Option<&(wgpu::TextureView, wgpu::TextureFormat)> {
        self.textures.get(handle)
    }

    pub fn contains_texture(&self, handle: &TextureHandle) -> bool { self.textures.contains_key(handle) }

    pub fn insert_texture_with_descriptor(
        &mut self,
        handle: TextureHandle,
        texture: wgpu::TextureView,
        descriptor: TextureResourceDescriptor,
        max_dimension: u32,
    ) -> Result<Option<(wgpu::TextureView, wgpu::TextureFormat)>, ResourceDescriptorError> {
        descriptor.validate(max_dimension)?;
        let old = self.textures.insert(handle, (texture, descriptor.format));
        self.owned_textures.remove(&handle);
        self.texture_descriptors.insert(handle, descriptor);
        self.bump_texture_version(handle);
        Ok(old)
    }

    pub fn texture_descriptor(&self, handle: &TextureHandle) -> Option<&TextureResourceDescriptor> {
        self.texture_descriptors.get(handle)
    }

    /// Lưu texture object thật cùng view compatibility. Đây là API cần cho
    /// copy/resolve; `insert_texture` cũ chỉ lưu view và không đủ ownership.
    pub fn insert_owned_texture(
        &mut self,
        handle: TextureHandle,
        texture: wgpu::Texture,
        descriptor: TextureResourceDescriptor,
        max_dimension: u32,
    ) -> Result<Option<OwnedTextureResource>, ResourceDescriptorError> {
        descriptor.validate(max_dimension)?;
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let old = self.owned_textures.insert(handle, OwnedTextureResource { texture, descriptor });
        self.textures.insert(handle, (view, descriptor.format));
        self.texture_descriptors.insert(handle, descriptor);
        self.bump_texture_version(handle);
        Ok(old)
    }

    pub fn owned_texture(&self, handle: &TextureHandle) -> Option<&wgpu::Texture> {
        self.owned_textures.get(handle).map(OwnedTextureResource::texture)
    }

    pub fn remove_owned_texture(&mut self, handle: &TextureHandle) -> Option<OwnedTextureResource> {
        let old = self.owned_textures.remove(handle);
        if old.is_some() {
            self.textures.remove(handle);
            self.texture_descriptors.remove(handle);
            self.bump_texture_version(*handle);
        }
        old
    }

    /// Tách texture khỏi registry nhưng giữ backing object tới sau submission
    /// cuối cùng dùng nó. Caller vẫn phải drain queue sau khi tracker báo hoàn tất.
    pub fn defer_owned_texture_destruction(
        &mut self,
        handle: &TextureHandle,
        last_use: SubmissionId,
        queue: &mut DeferredDestructionQueue<OwnedTextureResource>,
    ) -> bool {
        let Some(resource) = self.remove_owned_texture(handle) else { return false; };
        queue.defer(resource, last_use);
        true
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
        self.pipeline_layout_descriptors.remove(&handle);
        self.bump_pipeline_version(handle);
        old
    }

    pub fn insert_pipeline_with_layout_descriptor(
        &mut self,
        handle: PipelineHandle,
        pipeline: wgpu::RenderPipeline,
        descriptor: PipelineLayoutResourceDescriptor,
    ) -> Option<wgpu::RenderPipeline> {
        let old = self.pipelines.insert(handle, pipeline);
        self.pipeline_layout_descriptors.insert(handle, descriptor);
        self.bump_pipeline_version(handle);
        old
    }

    pub fn pipeline(&self, handle: &PipelineHandle) -> Option<&wgpu::RenderPipeline> {
        self.pipelines.get(handle)
    }

    pub fn contains_pipeline(&self, handle: &PipelineHandle) -> bool { self.pipelines.contains_key(handle) }

    pub fn pipeline_version(&self, handle: &PipelineHandle) -> ResourceVersion {
        self.versions.pipelines.get(handle).copied().unwrap_or(0)
    }

    pub fn pipeline_layout_descriptor(&self, handle: &PipelineHandle) -> Option<&PipelineLayoutResourceDescriptor> {
        self.pipeline_layout_descriptors.get(handle)
    }

    pub fn mark_pipeline_changed(&mut self, handle: PipelineHandle) {
        self.bump_pipeline_version(handle);
    }

    pub fn insert_compute_pipeline(
        &mut self,
        handle: ComputePipelineHandle,
        pipeline: wgpu::ComputePipeline,
    ) -> Option<wgpu::ComputePipeline> {
        let old = self.compute_pipelines.insert(handle, pipeline);
        self.compute_pipeline_layout_descriptors.remove(&handle);
        Self::bump_version(&mut self.versions.compute_pipelines, handle);
        old
    }

    pub fn insert_compute_pipeline_with_layout_descriptor(
        &mut self,
        handle: ComputePipelineHandle,
        pipeline: wgpu::ComputePipeline,
        descriptor: PipelineLayoutResourceDescriptor,
    ) -> Option<wgpu::ComputePipeline> {
        let old = self.compute_pipelines.insert(handle, pipeline);
        self.compute_pipeline_layout_descriptors.insert(handle, descriptor);
        Self::bump_version(&mut self.versions.compute_pipelines, handle);
        old
    }

    pub fn compute_pipeline(&self, handle: &ComputePipelineHandle) -> Option<&wgpu::ComputePipeline> {
        self.compute_pipelines.get(handle)
    }

    pub fn contains_compute_pipeline(&self, handle: &ComputePipelineHandle) -> bool { self.compute_pipelines.contains_key(handle) }

    pub fn compute_pipeline_version(&self, handle: &ComputePipelineHandle) -> ResourceVersion {
        self.versions.compute_pipelines.get(handle).copied().unwrap_or(0)
    }

    pub fn compute_pipeline_layout_descriptor(&self, handle: &ComputePipelineHandle) -> Option<&PipelineLayoutResourceDescriptor> {
        self.compute_pipeline_layout_descriptors.get(handle)
    }

    pub fn mark_compute_pipeline_changed(&mut self, handle: ComputePipelineHandle) {
        Self::bump_version(&mut self.versions.compute_pipelines, handle);
    }

    pub fn insert_buffer(&mut self, handle: BufferHandle, buffer: wgpu::Buffer) -> Option<wgpu::Buffer> {
        let old = self.buffers.insert(handle, buffer);
        self.buffer_descriptors.remove(&handle);
        Self::bump_version(&mut self.versions.buffers, handle);
        old
    }

    pub fn insert_buffer_with_descriptor(
        &mut self,
        handle: BufferHandle,
        buffer: wgpu::Buffer,
        descriptor: BufferResourceDescriptor,
    ) -> Result<Option<wgpu::Buffer>, BufferDescriptorError> {
        descriptor.validate()?;
        let old = self.buffers.insert(handle, buffer);
        self.buffer_descriptors.insert(handle, descriptor);
        Self::bump_version(&mut self.versions.buffers, handle);
        Ok(old)
    }

    pub fn buffer(&self, handle: &BufferHandle) -> Option<&wgpu::Buffer> { self.buffers.get(handle) }

    pub fn contains_buffer(&self, handle: &BufferHandle) -> bool { self.buffers.contains_key(handle) }

    pub fn mesh(&self, handle: &MeshHandle) -> Option<&(wgpu::Buffer, Option<(wgpu::Buffer, wgpu::IndexFormat)>, u32)> {
        self.meshes.get(handle)
    }

    pub fn contains_mesh(&self, handle: &MeshHandle) -> bool { self.meshes.contains_key(handle) }

    pub fn insert_mesh(
        &mut self,
        handle: MeshHandle,
        mesh: (wgpu::Buffer, Option<(wgpu::Buffer, wgpu::IndexFormat)>, u32),
    ) -> Option<(wgpu::Buffer, Option<(wgpu::Buffer, wgpu::IndexFormat)>, u32)> {
        let old = self.meshes.insert(handle, mesh);
        self.mesh_descriptors.remove(&handle);
        Self::bump_version(&mut self.versions.meshes, handle);
        old
    }

    pub fn insert_mesh_with_descriptor(
        &mut self,
        handle: MeshHandle,
        mesh: (wgpu::Buffer, Option<(wgpu::Buffer, wgpu::IndexFormat)>, u32),
        descriptor: MeshResourceDescriptor,
    ) -> Result<Option<(wgpu::Buffer, Option<(wgpu::Buffer, wgpu::IndexFormat)>, u32)>, MeshDescriptorError> {
        descriptor.validate()?;
        let old = self.meshes.insert(handle, mesh);
        self.mesh_descriptors.insert(handle, descriptor);
        Self::bump_version(&mut self.versions.meshes, handle);
        Ok(old)
    }

    pub fn mesh_descriptor(&self, handle: &MeshHandle) -> Option<&MeshResourceDescriptor> {
        self.mesh_descriptors.get(handle)
    }

    pub fn bind_group(&self, handle: &BindGroupHandle) -> Option<&wgpu::BindGroup> {
        self.bind_groups.get(handle)
    }

    pub fn contains_bind_group(&self, handle: &BindGroupHandle) -> bool { self.bind_groups.contains_key(handle) }

    pub fn insert_bind_group(&mut self, handle: BindGroupHandle, bind_group: wgpu::BindGroup) -> Option<wgpu::BindGroup> {
        let old = self.bind_groups.insert(handle, bind_group);
        self.bind_group_descriptors.remove(&handle);
        Self::bump_version(&mut self.versions.bind_groups, handle);
        old
    }

    pub fn insert_bind_group_with_descriptor(
        &mut self,
        handle: BindGroupHandle,
        bind_group: wgpu::BindGroup,
        descriptor: BindGroupResourceDescriptor,
    ) -> Result<Option<wgpu::BindGroup>, BindGroupDescriptorError> {
        descriptor.validate()?;
        let old = self.bind_groups.insert(handle, bind_group);
        self.bind_group_descriptors.insert(handle, descriptor);
        Self::bump_version(&mut self.versions.bind_groups, handle);
        Ok(old)
    }

    pub fn buffer_descriptor(&self, handle: &BufferHandle) -> Option<&BufferResourceDescriptor> {
        self.buffer_descriptors.get(handle)
    }

    pub fn buffer_version(&self, handle: &BufferHandle) -> ResourceVersion {
        self.versions.buffers.get(handle).copied().unwrap_or(0)
    }

    pub fn mark_buffer_changed(&mut self, handle: BufferHandle) {
        Self::bump_version(&mut self.versions.buffers, handle);
    }

    pub fn remove_buffer(&mut self, handle: &BufferHandle) -> Option<wgpu::Buffer> {
        let old = self.buffers.remove(handle);
        self.buffer_descriptors.remove(handle);
        if old.is_some() {
            Self::bump_version(&mut self.versions.buffers, *handle);
        }
        old
    }

    pub fn remove_compute_pipeline(&mut self, handle: &ComputePipelineHandle) -> Option<wgpu::ComputePipeline> {
        let old = self.compute_pipelines.remove(handle);
        self.compute_pipeline_layout_descriptors.remove(handle);
        if old.is_some() {
            Self::bump_version(&mut self.versions.compute_pipelines, *handle);
        }
        old
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

    pub fn bind_group_descriptor(&self, handle: &BindGroupHandle) -> Option<&BindGroupResourceDescriptor> {
        self.bind_group_descriptors.get(handle)
    }

    pub fn mark_bind_group_changed(&mut self, handle: BindGroupHandle) {
        Self::bump_version(&mut self.versions.bind_groups, handle);
    }

    pub fn remove_texture(&mut self, handle: &TextureHandle) -> Option<(wgpu::TextureView, wgpu::TextureFormat)> {
        let old = self.textures.remove(handle);
        self.owned_textures.remove(handle);
        self.texture_descriptors.remove(handle);
        if old.is_some() {
            self.bump_texture_version(*handle);
        }
        old
    }

    pub fn remove_pipeline(&mut self, handle: &PipelineHandle) -> Option<wgpu::RenderPipeline> {
        let old = self.pipelines.remove(handle);
        self.pipeline_layout_descriptors.remove(handle);
        if old.is_some() {
            self.bump_pipeline_version(*handle);
        }
        old
    }

    pub fn remove_mesh(&mut self, handle: &MeshHandle) -> Option<(wgpu::Buffer, Option<(wgpu::Buffer, wgpu::IndexFormat)>, u32)> {
        let old = self.meshes.remove(handle);
        self.mesh_descriptors.remove(handle);
        if old.is_some() {
            Self::bump_version(&mut self.versions.meshes, *handle);
        }
        old
    }

    pub fn remove_bind_group(&mut self, handle: &BindGroupHandle) -> Option<wgpu::BindGroup> {
        let old = self.bind_groups.remove(handle);
        self.bind_group_descriptors.remove(handle);
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
    use crate::api::GpuEngineBuilder;
    use crate::memory::{DeferredDestructionQueue, SubmissionTracker};

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

    #[test]
    fn compute_pipeline_versions_are_independent_from_render_pipelines() {
        let mut registry = ResourceRegistry::new();
        registry.mark_pipeline_changed(PipelineHandle(1));
        registry.mark_compute_pipeline_changed(ComputePipelineHandle(1));

        assert_eq!(registry.pipeline_version(&PipelineHandle(1)), 1);
        assert_eq!(registry.compute_pipeline_version(&ComputePipelineHandle(1)), 1);
    }

    #[test]
    fn buffer_versions_are_independent_from_texture_versions() {
        let mut registry = ResourceRegistry::new();
        registry.mark_buffer_changed(BufferHandle(1));
        registry.mark_texture_changed(TextureHandle(1));

        assert_eq!(registry.buffer_version(&BufferHandle(1)), 1);
        assert_eq!(registry.texture_version(&TextureHandle(1)), 1);
    }

    #[test]
    fn buffer_descriptor_rejects_invalid_size_and_usage() {
        assert_eq!(BufferResourceDescriptor { size: 0, usage: wgpu::BufferUsages::COPY_SRC }.validate(), Err(BufferDescriptorError::InvalidSize));
        assert_eq!(BufferResourceDescriptor { size: 4, usage: wgpu::BufferUsages::empty() }.validate(), Err(BufferDescriptorError::EmptyUsage));
        assert_eq!(BufferResourceDescriptor { size: 4, usage: wgpu::BufferUsages::COPY_SRC }.validate(), Ok(()));
    }

    #[test]
    fn mesh_descriptor_rejects_inconsistent_metadata() {
        assert_eq!(
            MeshResourceDescriptor { vertex_buffer_size: 0, vertex_count: 3, index_buffer_size: None, index_format: None }.validate(),
            Err(MeshDescriptorError::InvalidVertexBufferSize)
        );
        assert_eq!(
            MeshResourceDescriptor { vertex_buffer_size: 4, vertex_count: 0, index_buffer_size: None, index_format: None }.validate(),
            Err(MeshDescriptorError::InvalidVertexCount)
        );
        assert_eq!(
            MeshResourceDescriptor { vertex_buffer_size: 4, vertex_count: 3, index_buffer_size: Some(0), index_format: Some(wgpu::IndexFormat::Uint16) }.validate(),
            Err(MeshDescriptorError::InvalidIndexBufferSize)
        );
        assert_eq!(
            MeshResourceDescriptor { vertex_buffer_size: 4, vertex_count: 3, index_buffer_size: None, index_format: Some(wgpu::IndexFormat::Uint16) }.validate(),
            Err(MeshDescriptorError::IndexFormatWithoutBuffer)
        );
        assert_eq!(
            MeshResourceDescriptor { vertex_buffer_size: 4, vertex_count: 3, index_buffer_size: Some(6), index_format: Some(wgpu::IndexFormat::Uint16) }.validate(),
            Ok(())
        );
    }

    #[test]
    fn bind_group_descriptor_validates_dynamic_offset_contract() {
        assert_eq!(
            BindGroupResourceDescriptor { dynamic_offset_count: 0, dynamic_offset_alignment: 0, layout_signature: 7 }.validate(),
            Ok(())
        );
        assert_eq!(
            BindGroupResourceDescriptor { dynamic_offset_count: 0, dynamic_offset_alignment: 256, layout_signature: 7 }.validate(),
            Err(BindGroupDescriptorError::UnexpectedAlignmentWithoutOffsets)
        );
        assert_eq!(
            BindGroupResourceDescriptor { dynamic_offset_count: 1, dynamic_offset_alignment: 0, layout_signature: 7 }.validate(),
            Err(BindGroupDescriptorError::InvalidAlignment)
        );
        assert_eq!(
            BindGroupResourceDescriptor { dynamic_offset_count: 2, dynamic_offset_alignment: 256, layout_signature: 7 }.validate(),
            Ok(())
        );
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

    #[test]
    fn texture_descriptor_rejects_impossible_mips_and_sample_count() {
        let mut descriptor = valid_descriptor();
        descriptor.width = 8;
        descriptor.height = 4;
        descriptor.mip_level_count = 5;
        assert_eq!(
            descriptor.validate(1024),
            Err(ResourceDescriptorError::MipCountExceedsExtent {
                mip_level_count: 5,
                max_mip_level_count: 4,
                width: 8,
                height: 4,
            })
        );

        descriptor = valid_descriptor();
        descriptor.sample_count = 3;
        assert_eq!(
            descriptor.validate(1024),
            Err(ResourceDescriptorError::InvalidSampleCountValue { sample_count: 3 })
        );
    }

    #[test]
    fn owned_texture_keeps_texture_object_and_descriptor_together() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("owned_texture_test"),
            size: wgpu::Extent3d { width: 16, height: 8, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let mut registry = ResourceRegistry::new();
        let descriptor = TextureResourceDescriptor {
            width: 16, height: 8, depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            mip_level_count: 1, sample_count: 1,
        };

        registry.insert_owned_texture(TextureHandle(3), texture, descriptor, 1024).unwrap();
        assert!(registry.owned_texture(&TextureHandle(3)).is_some());
        assert_eq!(registry.texture_descriptor(&TextureHandle(3)), Some(&descriptor));
        assert!(registry.texture(&TextureHandle(3)).is_some());
        assert!(registry.remove_owned_texture(&TextureHandle(3)).is_some());
        assert!(registry.owned_texture(&TextureHandle(3)).is_none());
        assert!(registry.texture(&TextureHandle(3)).is_none());
    }

    #[test]
    fn owned_texture_deferred_removal_waits_for_submission_completion() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("deferred_owned_texture_test"),
            size: wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let descriptor = TextureResourceDescriptor {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            mip_level_count: 1,
            sample_count: 1,
        };
        let mut registry = ResourceRegistry::new();
        registry.insert_owned_texture(TextureHandle(9), texture, descriptor, 1024).unwrap();
        let mut tracker = SubmissionTracker::new();
        let last_use = tracker.begin();
        let mut queue = DeferredDestructionQueue::new();
        assert!(registry.defer_owned_texture_destruction(&TextureHandle(9), last_use, &mut queue));
        assert!(registry.owned_texture(&TextureHandle(9)).is_none());
        assert_eq!(queue.pending_count(), 1);
        assert!(queue.drain_completed(&tracker).is_empty());
        tracker.mark_completed(last_use);
        assert_eq!(queue.drain_completed(&tracker).len(), 1);
        assert!(!registry.defer_owned_texture_destruction(&TextureHandle(9), last_use, &mut queue));
    }
}
