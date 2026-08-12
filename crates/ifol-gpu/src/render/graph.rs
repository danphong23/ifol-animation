use std::collections::HashMap;
use std::ops::Range;
use crate::render::handle::{BindGroupHandle, MeshHandle, PipelineHandle, RenderNodeId, TextureHandle};

/// ═══════════════════════════════════════════════════════════
/// HÀNH ĐỘNG VẼ (DrawAction)
/// Ánh xạ trực tiếp sang wgpu::RenderPass::draw* methods
/// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawAction {
    /// Vẽ theo hình dáng Mesh có sẵn trong VRAM (Vertex + Index Buffer).
    /// → Ánh xạ: pass.set_vertex_buffer() + pass.set_index_buffer() + pass.draw_indexed()
    Indexed {
        mesh: MeshHandle,
        index_range: Range<u32>,
        instance_range: Range<u32>,
    },

    /// Vẽ không cần Mesh — Shader tự tạo đỉnh từ vertex_index.
    /// → Ánh xạ: pass.draw(0..vertex_count, instance_range)
    Procedural {
        vertex_count: u32,
        instance_range: Range<u32>,
    },
}

/// ═══════════════════════════════════════════════════════════
/// LỆNH VẼ HOÀN CHỈNH (DrawCommand)
/// Mỗi lệnh = 1 lần quẹt cọ trên bức tranh
/// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawCommand {
    /// Shader quyết định cách tô màu pixel
    pub pipeline: PipelineHandle,

    /// Danh sách túi dữ liệu: Vec<(Slot_Index, BindGroupHandle, Dynamic_Offsets)>
    pub bind_groups: Vec<(u32, BindGroupHandle, Vec<u32>)>,

    /// Hành động quẹt cọ cụ thể (Indexed hoặc Procedural)
    pub action: DrawAction,
}

impl DrawCommand {
    pub fn new(pipeline: PipelineHandle, action: DrawAction) -> Self {
        Self {
            pipeline,
            bind_groups: Vec::new(),
            action,
        }
    }

    pub fn with_bind_group(mut self, slot: u32, handle: BindGroupHandle, offsets: Vec<u32>) -> Self {
        self.bind_groups.push((slot, handle, offsets));
        self
    }
}

/// ═══════════════════════════════════════════════════════════
/// ĐÍCH ĐẾN (RenderTarget) — "Bức tranh sẽ in lên đâu?"
/// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderTarget {
    /// In thẳng ra cửa sổ hệ điều hành (Swap Chain)
    Screen,

    /// In ra một tấm ảnh ảo trong VRAM với kích thước chính xác
    Offscreen {
        color: TextureHandle,
        width: u32,
        height: u32,
    },
}

/// ═══════════════════════════════════════════════════════════
/// NÚT VẼ (RenderNode) — "Một hành động trên bức tranh"
/// ═══════════════════════════════════════════════════════════

#[derive(Debug)]
pub enum RenderNode {
    /// Nhóm đệ quy (Pre-comp / Group / Camera Post-FX).
    /// Vẽ graph con ra Offscreen trước, sau đó thực thi commands để in kết quả lên Graph cha.
    SubGraph {
        name: String,
        graph: Box<RenderGraph>,
        commands: Vec<DrawCommand>,
        is_dirty: bool,
        bundle: Option<wgpu::RenderBundle>,
    },

    /// Danh sách lệnh vẽ phẳng trên cùng 1 target.
    DrawBatch {
        commands: Vec<DrawCommand>,
        is_dirty: bool,
        bundle: Option<wgpu::RenderBundle>,
    },
}

impl RenderNode {
    pub fn new_batch(commands: Vec<DrawCommand>) -> Self {
        Self::DrawBatch {
            commands,
            is_dirty: true,
            bundle: None,
        }
    }

    pub fn new_subgraph(name: impl Into<String>, graph: RenderGraph, commands: Vec<DrawCommand>) -> Self {
        Self::SubGraph {
            name: name.into(),
            graph: Box::new(graph),
            commands,
            is_dirty: true,
            bundle: None,
        }
    }

    pub fn commands(&self) -> &[DrawCommand] {
        match self {
            Self::SubGraph { commands, .. } => commands,
            Self::DrawBatch { commands, .. } => commands,
        }
    }

    pub fn is_dirty(&self) -> bool {
        match self {
            Self::SubGraph { is_dirty, .. } => *is_dirty,
            Self::DrawBatch { is_dirty, .. } => *is_dirty,
        }
    }

    pub fn bundle(&self) -> Option<&wgpu::RenderBundle> {
        match self {
            Self::SubGraph { bundle, .. } => bundle.as_ref(),
            Self::DrawBatch { bundle, .. } => bundle.as_ref(),
        }
    }
}

/// ═══════════════════════════════════════════════════════════
/// ARENA POOL (RenderNodePool) — "Nơi lưu trữ các Nút Vẽ"
/// ═══════════════════════════════════════════════════════════

#[derive(Default)]
pub struct RenderNodePool {
    nodes: HashMap<RenderNodeId, RenderNode>,
    next_id: u64,
}

impl RenderNodePool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc_batch(&mut self, commands: Vec<DrawCommand>) -> RenderNodeId {
        self.next_id += 1;
        let id = RenderNodeId(self.next_id);
        self.nodes.insert(id, RenderNode::new_batch(commands));
        id
    }

    pub fn alloc_subgraph(&mut self, name: impl Into<String>, graph: RenderGraph, commands: Vec<DrawCommand>) -> RenderNodeId {
        self.next_id += 1;
        let id = RenderNodeId(self.next_id);
        self.nodes.insert(id, RenderNode::new_subgraph(name, graph, commands));
        id
    }

    pub fn get(&self, id: RenderNodeId) -> Option<&RenderNode> {
        self.nodes.get(&id)
    }

    pub fn get_mut(&mut self, id: RenderNodeId) -> Option<&mut RenderNode> {
        self.nodes.get_mut(&id)
    }

    pub fn update_commands(&mut self, id: RenderNodeId, commands: Vec<DrawCommand>) -> bool {
        if let Some(node) = self.nodes.get_mut(&id) {
            match node {
                RenderNode::DrawBatch { commands: cmds, is_dirty, bundle } => {
                    *cmds = commands;
                    *is_dirty = true;
                    *bundle = None;
                }
                RenderNode::SubGraph { commands: cmds, is_dirty, bundle, .. } => {
                    *cmds = commands;
                    *is_dirty = true;
                    *bundle = None;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn mark_dirty(&mut self, id: RenderNodeId) {
        if let Some(node) = self.nodes.get_mut(&id) {
            match node {
                RenderNode::DrawBatch { is_dirty, bundle, .. } |
                RenderNode::SubGraph { is_dirty, bundle, .. } => {
                    *is_dirty = true;
                    *bundle = None;
                }
            }
        }
    }
}

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
}

impl RenderGraph {
    pub fn new(target: RenderTarget) -> Self {
        Self {
            target,
            clear_color: None,
            depth_stencil: None,
            node_ids: Vec::new(),
        }
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

    pub fn add_batch(&mut self, pool: &mut RenderNodePool, commands: Vec<DrawCommand>) -> RenderNodeId {
        let id = pool.alloc_batch(commands);
        self.node_ids.push(id);
        id
    }

    pub fn add_subgraph(&mut self, pool: &mut RenderNodePool, name: impl Into<String>, graph: RenderGraph, commands: Vec<DrawCommand>) -> RenderNodeId {
        let id = pool.alloc_subgraph(name, graph, commands);
        self.node_ids.push(id);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_graph_nesting() {
        let mut pool = RenderNodePool::new();

        let mut shadow_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: TextureHandle(1),
            width: 2048,
            height: 2048,
        })
        .with_depth_stencil(TextureHandle(2));

        let shadow_batch_id = pool.alloc_batch(vec![DrawCommand::new(
            PipelineHandle(10),
            DrawAction::Indexed {
                mesh: MeshHandle(100),
                index_range: 0..36,
                instance_range: 0..1,
            },
        )]);
        shadow_graph.add_node_id(shadow_batch_id);

        let mut root_graph = RenderGraph::new(RenderTarget::Screen)
            .with_clear_color([0.1, 0.1, 0.1, 1.0])
            .with_depth_stencil(TextureHandle(3));

        // SubGraph Shadow Map (không có command in lên màn hình)
        let sub_id = pool.alloc_subgraph("ShadowPass", shadow_graph, vec![]);
        root_graph.add_node_id(sub_id);

        // DrawBatch chính
        let main_batch_id = pool.alloc_batch(vec![DrawCommand::new(
            PipelineHandle(20),
            DrawAction::Indexed {
                mesh: MeshHandle(200),
                index_range: 0..12,
                instance_range: 0..1,
            },
        )]);
        root_graph.add_node_id(main_batch_id);

        assert_eq!(root_graph.node_ids.len(), 2);
        match pool.get(root_graph.node_ids[0]).unwrap() {
            RenderNode::SubGraph { name, graph, commands, .. } => {
                assert_eq!(name, "ShadowPass");
                assert_eq!(graph.node_ids.len(), 1);
                assert!(commands.is_empty());
            }
            _ => panic!("Kỳ vọng Node 0 là SubGraph"),
        }
    }
}
