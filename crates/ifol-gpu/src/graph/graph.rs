use super::flatten::GraphDependency;
use super::{
    ComputeCommand, CopyCommand, DrawCommand, RenderNodePool, RenderTarget, ResourceUsage,
};
use crate::resources::handle::{RenderNodeId, TextureHandle};
use std::collections::HashMap;

/// ═══════════════════════════════════════════════════════════
/// ĐỒ THỊ VẼ (RenderGraph) — "Tấm toan chứa danh sách ID Nút vẽ"
/// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct RenderGraph {
    /// Bức tranh này sẽ được in ra đâu?
    pub target: RenderTarget,

    /// Xóa phông nền trước khi vẽ (None = vẽ đè lên nội dung cũ)
    pub clear_color: Option<[f32; 4]>,

    /// [3D-Ready] Depth/Stencil Texture dùng chung cho toàn bộ Graph này
    pub depth_stencil: Option<TextureHandle>,

    /// Danh sách ID các nút vẽ trong RenderNodePool. Thứ tự 0 → N = thứ tự vẽ đè
    pub node_ids: Vec<RenderNodeId>,

    /// Duyệt từ cuối lên đầu thay vì từ đầu tới cuối (Reverse Draw Order)
    pub reverse_draw_order: bool,
    pub dependencies: Vec<GraphDependency>,
    pub(crate) resource_usages: HashMap<RenderNodeId, Vec<ResourceUsage>>,
}

impl RenderGraph {
    pub fn new(target: RenderTarget) -> Self {
        Self {
            target,
            clear_color: None,
            depth_stencil: None,
            node_ids: Vec::new(),
            reverse_draw_order: false,
            dependencies: Vec::new(),
            resource_usages: HashMap::new(),
        }
    }

    pub fn with_reverse_draw_order(mut self, reverse: bool) -> Self {
        self.reverse_draw_order = reverse;
        self
    }

    pub fn with_clear_color(mut self, color: [f32; 4]) -> Self {
        self.clear_color = Some(color);
        self
    }

    pub fn with_depth_stencil(mut self, handle: TextureHandle) -> Self {
        self.depth_stencil = Some(handle);
        self
    }

    pub fn add_node_id(&mut self, id: RenderNodeId) {
        self.node_ids.push(id);
    }

    pub fn add_dependency(&mut self, before: RenderNodeId, after: RenderNodeId) {
        self.dependencies.push(GraphDependency { before, after });
    }

    pub fn add_batch(
        &mut self,
        pool: &mut RenderNodePool,
        commands: Vec<DrawCommand>,
    ) -> RenderNodeId {
        let id = pool.alloc_batch(commands);
        self.node_ids.push(id);
        id
    }

    pub fn add_compute_batch(
        &mut self,
        pool: &mut RenderNodePool,
        commands: Vec<ComputeCommand>,
    ) -> RenderNodeId {
        let id = pool.alloc_compute_batch(commands);
        self.node_ids.push(id);
        id
    }

    pub fn add_copy_batch(
        &mut self,
        pool: &mut RenderNodePool,
        commands: Vec<CopyCommand>,
    ) -> RenderNodeId {
        let id = pool.alloc_copy_batch(commands);
        self.node_ids.push(id);
        id
    }

    pub fn add_subgraph(
        &mut self,
        pool: &mut RenderNodePool,
        name: impl Into<String>,
        graph: RenderGraph,
        commands: Vec<DrawCommand>,
    ) -> RenderNodeId {
        let id = pool.alloc_subgraph(name, graph, commands);
        self.node_ids.push(id);
        id
    }

}
