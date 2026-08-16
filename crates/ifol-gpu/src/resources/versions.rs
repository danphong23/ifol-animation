use crate::resources::handle::{
    BindGroupHandle, BufferHandle, ComputePipelineHandle, MeshHandle, PipelineHandle, TextureHandle,
};
use crate::resources::registry::ResourceRegistry;
use std::collections::HashMap;

pub(crate) type ResourceVersion = u64;

#[derive(Default)]
pub(crate) struct ResourceVersions {
    pub(crate) textures: HashMap<TextureHandle, ResourceVersion>,
    pub(crate) pipelines: HashMap<PipelineHandle, ResourceVersion>,
    pub(crate) compute_pipelines: HashMap<ComputePipelineHandle, ResourceVersion>,
    pub(crate) buffers: HashMap<BufferHandle, ResourceVersion>,
    pub(crate) meshes: HashMap<MeshHandle, ResourceVersion>,
    pub(crate) bind_groups: HashMap<BindGroupHandle, ResourceVersion>,
}

impl ResourceRegistry {
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
