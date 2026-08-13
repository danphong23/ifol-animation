use std::collections::HashMap;
use crate::render::handle::{BindGroupHandle, MeshHandle, PipelineHandle, TextureHandle};

type ResourceVersion = u64;

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
}
