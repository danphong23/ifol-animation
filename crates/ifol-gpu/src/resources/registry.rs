pub use super::descriptors::*;
pub use super::ownership::OwnedTextureResource;
use super::versions::{ResourceVersion, ResourceVersions};
use crate::resources::handle::{
    BindGroupHandle, BufferHandle, ComputePipelineHandle, MeshHandle, PipelineHandle, TextureHandle,
};
use std::collections::HashMap;

/// Nơi ánh xạ từ Handle siêu nhẹ (u64) sang các đối tượng nặng của GPU (Buffer, Texture, Pipeline)
#[derive(Default)]
pub struct ResourceRegistry {
    pub(super) textures: HashMap<TextureHandle, (wgpu::TextureView, wgpu::TextureFormat)>,
    pub(super) pipelines: HashMap<PipelineHandle, wgpu::RenderPipeline>,
    pub(super) compute_pipelines: HashMap<ComputePipelineHandle, wgpu::ComputePipeline>,
    pub(super) pipeline_layout_descriptors:
        HashMap<PipelineHandle, PipelineLayoutResourceDescriptor>,
    pub(super) compute_pipeline_layout_descriptors:
        HashMap<ComputePipelineHandle, PipelineLayoutResourceDescriptor>,
    pub(super) buffers: HashMap<BufferHandle, wgpu::Buffer>,
    /// Lưu trữ Mesh: (VBO, Option<(IBO, IndexFormat)>, Số lượng Index/Vertex mặc định)
    pub(super) meshes:
        HashMap<MeshHandle, (wgpu::Buffer, Option<(wgpu::Buffer, wgpu::IndexFormat)>, u32)>,
    pub(super) mesh_descriptors: HashMap<MeshHandle, MeshResourceDescriptor>,
    pub(super) bind_groups: HashMap<BindGroupHandle, wgpu::BindGroup>,
    pub(super) bind_group_descriptors: HashMap<BindGroupHandle, BindGroupResourceDescriptor>,
    pub(super) buffer_descriptors: HashMap<BufferHandle, BufferResourceDescriptor>,
    pub(super) texture_descriptors: HashMap<TextureHandle, TextureResourceDescriptor>,
    pub(super) owned_textures: HashMap<TextureHandle, OwnedTextureResource>,
    pub(super) versions: ResourceVersions,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn texture_version(&self, handle: &TextureHandle) -> ResourceVersion {
        self.versions.textures.get(handle).copied().unwrap_or(0)
    }

    pub fn mark_texture_changed(&mut self, handle: TextureHandle) {
        self.bump_texture_version(handle);
    }

    pub fn pipeline_version(&self, handle: &PipelineHandle) -> ResourceVersion {
        self.versions.pipelines.get(handle).copied().unwrap_or(0)
    }

    pub fn mark_pipeline_changed(&mut self, handle: PipelineHandle) {
        self.bump_pipeline_version(handle);
    }

    pub fn compute_pipeline_version(&self, handle: &ComputePipelineHandle) -> ResourceVersion {
        self.versions
            .compute_pipelines
            .get(handle)
            .copied()
            .unwrap_or(0)
    }

    pub fn mark_compute_pipeline_changed(&mut self, handle: ComputePipelineHandle) {
        Self::bump_version(&mut self.versions.compute_pipelines, handle);
    }

    pub fn buffer_version(&self, handle: &BufferHandle) -> ResourceVersion {
        self.versions.buffers.get(handle).copied().unwrap_or(0)
    }

    pub fn mark_buffer_changed(&mut self, handle: BufferHandle) {
        Self::bump_version(&mut self.versions.buffers, handle);
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

    pub(super) fn bump_texture_version(&mut self, handle: TextureHandle) {
        Self::bump_version(&mut self.versions.textures, handle);
    }

    pub(super) fn bump_pipeline_version(&mut self, handle: PipelineHandle) {
        Self::bump_version(&mut self.versions.pipelines, handle);
    }

    pub(super) fn bump_version<H: Copy + Eq + std::hash::Hash>(
        versions: &mut HashMap<H, ResourceVersion>,
        handle: H,
    ) {
        let version = versions.entry(handle).or_insert(0);
        *version = version.saturating_add(1);
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
