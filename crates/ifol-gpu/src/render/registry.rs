use std::collections::HashMap;
use crate::render::handle::{BindGroupHandle, MeshHandle, PipelineHandle, TextureHandle};

/// Nơi ánh xạ từ Handle siêu nhẹ (u64) sang các đối tượng nặng của GPU (Buffer, Texture, Pipeline)
#[derive(Default)]
pub struct ResourceRegistry {
    pub textures: HashMap<TextureHandle, wgpu::TextureView>,
    pub pipelines: HashMap<PipelineHandle, wgpu::RenderPipeline>,
    /// Lưu trữ Mesh (VBO, Option<IBO>, Số lượng Index/Vertex để vẽ)
    pub meshes: HashMap<MeshHandle, (wgpu::Buffer, Option<wgpu::Buffer>, u32)>, 
    pub bind_groups: HashMap<BindGroupHandle, wgpu::BindGroup>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn remove_texture(&mut self, handle: &TextureHandle) -> Option<wgpu::TextureView> {
        self.textures.remove(handle)
    }

    pub fn remove_pipeline(&mut self, handle: &PipelineHandle) -> Option<wgpu::RenderPipeline> {
        self.pipelines.remove(handle)
    }

    pub fn remove_mesh(&mut self, handle: &MeshHandle) -> Option<(wgpu::Buffer, Option<wgpu::Buffer>, u32)> {
        self.meshes.remove(handle)
    }

    pub fn remove_bind_group(&mut self, handle: &BindGroupHandle) -> Option<wgpu::BindGroup> {
        self.bind_groups.remove(handle)
    }
}
