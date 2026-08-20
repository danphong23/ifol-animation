use super::descriptors::{
    BindGroupDescriptorError, BindGroupResourceDescriptor, BufferDescriptorError,
    BufferResourceDescriptor, MeshDescriptorError, MeshResourceDescriptor,
    PipelineLayoutResourceDescriptor, ResourceDescriptorError, TextureResourceDescriptor,
};
use super::{MeshResource, ResourceRegistry};
use crate::resources::handle::{
    BindGroupHandle, BufferHandle, ComputePipelineHandle, MeshHandle, PipelineHandle, TextureHandle,
};

impl ResourceRegistry {
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

    pub fn insert_compute_pipeline_with_layout_descriptor(
        &mut self,
        handle: ComputePipelineHandle,
        pipeline: wgpu::ComputePipeline,
        descriptor: PipelineLayoutResourceDescriptor,
    ) -> Option<wgpu::ComputePipeline> {
        let old = self.compute_pipelines.insert(handle, pipeline);
        self.compute_pipeline_layout_descriptors
            .insert(handle, descriptor);
        Self::bump_version(&mut self.versions.compute_pipelines, handle);
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

    pub fn insert_mesh_with_descriptor(
        &mut self,
        handle: MeshHandle,
        mesh: MeshResource,
        descriptor: MeshResourceDescriptor,
    ) -> Result<
        Option<MeshResource>,
        MeshDescriptorError,
    > {
        descriptor.validate()?;
        let old = self.meshes.insert(handle, mesh);
        self.mesh_descriptors.insert(handle, descriptor);
        Self::bump_version(&mut self.versions.meshes, handle);
        Ok(old)
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

    pub fn remove_buffer(&mut self, handle: &BufferHandle) -> Option<wgpu::Buffer> {
        let old = self.buffers.remove(handle);
        self.buffer_descriptors.remove(handle);
        if old.is_some() {
            Self::bump_version(&mut self.versions.buffers, *handle);
        }
        old
    }

    pub fn remove_compute_pipeline(
        &mut self,
        handle: &ComputePipelineHandle,
    ) -> Option<wgpu::ComputePipeline> {
        let old = self.compute_pipelines.remove(handle);
        self.compute_pipeline_layout_descriptors.remove(handle);
        if old.is_some() {
            Self::bump_version(&mut self.versions.compute_pipelines, *handle);
        }
        old
    }

    pub fn remove_texture(
        &mut self,
        handle: &TextureHandle,
    ) -> Option<(wgpu::TextureView, wgpu::TextureFormat)> {
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

    pub fn remove_mesh(
        &mut self,
        handle: &MeshHandle,
    ) -> Option<MeshResource> {
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
}
