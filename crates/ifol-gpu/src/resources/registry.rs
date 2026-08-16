pub use super::descriptors::*;
pub use super::ownership::OwnedTextureResource;
use super::versions::ResourceVersions;
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
}

#[cfg(test)]
#[path = "registry_version_tests.rs"]
mod version_tests;
#[cfg(test)]
#[path = "registry_descriptor_tests.rs"]
mod descriptor_tests;
#[cfg(test)]
#[path = "registry_ownership_tests.rs"]
mod ownership_tests;
