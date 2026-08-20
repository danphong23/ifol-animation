use super::descriptors::{
    BindGroupResourceDescriptor, BufferResourceDescriptor, MeshResourceDescriptor,
    PipelineLayoutResourceDescriptor, TextureResourceDescriptor,
};
use super::{MeshResource, OwnedTextureResource, ResourceRegistry};
use crate::resources::handle::{
    BindGroupHandle, BufferHandle, ComputePipelineHandle, MeshHandle, PipelineHandle, TextureHandle,
};

impl ResourceRegistry {
    pub fn texture(
        &self,
        handle: &TextureHandle,
    ) -> Option<&(wgpu::TextureView, wgpu::TextureFormat)> {
        self.textures.get(handle)
    }

    pub fn contains_texture(&self, handle: &TextureHandle) -> bool {
        self.textures.contains_key(handle)
    }

    pub fn texture_descriptor(&self, handle: &TextureHandle) -> Option<&TextureResourceDescriptor> {
        self.texture_descriptors.get(handle)
    }

    pub fn texture_format(&self, handle: &TextureHandle) -> Option<wgpu::TextureFormat> {
        self.texture_descriptors.get(handle).map(|descriptor| descriptor.format)
    }

    pub fn owned_texture(&self, handle: &TextureHandle) -> Option<&wgpu::Texture> {
        self.owned_textures
            .get(handle)
            .map(OwnedTextureResource::texture)
    }

    pub fn pipeline(&self, handle: &PipelineHandle) -> Option<&wgpu::RenderPipeline> {
        self.pipelines.get(handle)
    }

    pub fn contains_pipeline(&self, handle: &PipelineHandle) -> bool {
        self.pipelines.contains_key(handle)
    }

    pub fn pipeline_layout_descriptor(
        &self,
        handle: &PipelineHandle,
    ) -> Option<&PipelineLayoutResourceDescriptor> {
        self.pipeline_layout_descriptors.get(handle)
    }

    pub fn compute_pipeline(
        &self,
        handle: &ComputePipelineHandle,
    ) -> Option<&wgpu::ComputePipeline> {
        self.compute_pipelines.get(handle)
    }

    pub fn contains_compute_pipeline(&self, handle: &ComputePipelineHandle) -> bool {
        self.compute_pipelines.contains_key(handle)
    }

    pub fn compute_pipeline_layout_descriptor(
        &self,
        handle: &ComputePipelineHandle,
    ) -> Option<&PipelineLayoutResourceDescriptor> {
        self.compute_pipeline_layout_descriptors.get(handle)
    }

    pub fn buffer(&self, handle: &BufferHandle) -> Option<&wgpu::Buffer> {
        self.buffers.get(handle)
    }

    pub fn contains_buffer(&self, handle: &BufferHandle) -> bool {
        self.buffers.contains_key(handle)
    }

    pub fn buffer_descriptor(&self, handle: &BufferHandle) -> Option<&BufferResourceDescriptor> {
        self.buffer_descriptors.get(handle)
    }

    pub fn mesh(
        &self,
        handle: &MeshHandle,
    ) -> Option<&MeshResource> {
        self.meshes.get(handle)
    }

    pub fn contains_mesh(&self, handle: &MeshHandle) -> bool {
        self.meshes.contains_key(handle)
    }

    pub fn mesh_descriptor(&self, handle: &MeshHandle) -> Option<&MeshResourceDescriptor> {
        self.mesh_descriptors.get(handle)
    }

    pub fn bind_group(&self, handle: &BindGroupHandle) -> Option<&wgpu::BindGroup> {
        self.bind_groups.get(handle)
    }

    pub fn contains_bind_group(&self, handle: &BindGroupHandle) -> bool {
        self.bind_groups.contains_key(handle)
    }

    pub fn bind_group_descriptor(
        &self,
        handle: &BindGroupHandle,
    ) -> Option<&BindGroupResourceDescriptor> {
        self.bind_group_descriptors.get(handle)
    }
}
