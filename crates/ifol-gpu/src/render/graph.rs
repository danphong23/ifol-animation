use std::ops::Range;
use crate::render::handle::{BindGroupHandle, MeshHandle, PipelineHandle, TextureHandle};

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

#[derive(Debug, Clone)]
pub enum RenderNode {
    /// Nhóm đệ quy (Pre-comp / Group / Camera Post-FX).
    /// Vẽ graph con ra Offscreen trước, sau đó thực thi commands để in kết quả lên Graph cha.
    SubGraph {
        name: String,
        graph: Box<RenderGraph>,
        commands: Vec<DrawCommand>,
        is_dirty: bool,
    },

    /// Danh sách lệnh vẽ phẳng trên cùng 1 target.
    DrawBatch {
        commands: Vec<DrawCommand>,
        is_dirty: bool,
    },
}

impl RenderNode {
    pub fn new_batch(commands: Vec<DrawCommand>) -> Self {
        Self::DrawBatch {
            commands,
            is_dirty: true,
        }
    }

    pub fn new_subgraph(name: impl Into<String>, graph: RenderGraph, commands: Vec<DrawCommand>) -> Self {
        Self::SubGraph {
            name: name.into(),
            graph: Box::new(graph),
            commands,
            is_dirty: true,
        }
    }
}

/// ═══════════════════════════════════════════════════════════
/// ĐỒ THỊ VẼ (RenderGraph) — "Tấm toan chứa các nét cọ"
/// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct RenderGraph {
    /// Bức tranh này sẽ được in ra đâu?
    pub target: RenderTarget,

    /// Xóa phông nền trước khi vẽ (None = vẽ đè lên nội dung cũ)
    pub clear_color: Option<[f32; 4]>,

    /// [3D-Ready] Depth/Stencil Texture dùng chung cho toàn bộ Graph này
    pub depth_stencil: Option<TextureHandle>,

    /// Danh sách các nút vẽ. Thứ tự 0 → N = thứ tự vẽ đè
    pub nodes: Vec<RenderNode>,
}

impl RenderGraph {
    pub fn new(target: RenderTarget) -> Self {
        Self {
            target,
            clear_color: None,
            depth_stencil: None,
            nodes: Vec::new(),
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

    pub fn add_node(&mut self, node: RenderNode) {
        self.nodes.push(node);
    }

    pub fn add_batch(&mut self, commands: Vec<DrawCommand>) {
        self.nodes.push(RenderNode::new_batch(commands));
    }

    pub fn add_subgraph(&mut self, name: impl Into<String>, graph: RenderGraph, commands: Vec<DrawCommand>) {
        self.nodes.push(RenderNode::new_subgraph(name, graph, commands));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_graph_nesting() {
        let mut shadow_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: TextureHandle(1),
            width: 2048,
            height: 2048,
        })
        .with_depth_stencil(TextureHandle(2));

        shadow_graph.add_batch(vec![DrawCommand::new(
            PipelineHandle(10),
            DrawAction::Indexed {
                mesh: MeshHandle(100),
                index_range: 0..36,
                instance_range: 0..1,
            },
        )]);

        let mut root_graph = RenderGraph::new(RenderTarget::Screen)
            .with_clear_color([0.1, 0.1, 0.1, 1.0])
            .with_depth_stencil(TextureHandle(3));

        // SubGraph Shadow Map (không có command in lên màn hình)
        root_graph.add_subgraph("ShadowPass", shadow_graph, vec![]);

        // DrawBatch chính
        root_graph.add_batch(vec![DrawCommand::new(
            PipelineHandle(20),
            DrawAction::Indexed {
                mesh: MeshHandle(200),
                index_range: 0..12,
                instance_range: 0..1,
            },
        )]);

        assert_eq!(root_graph.nodes.len(), 2);
        match &root_graph.nodes[0] {
            RenderNode::SubGraph { name, graph, commands, .. } => {
                assert_eq!(name, "ShadowPass");
                assert_eq!(graph.nodes.len(), 1);
                assert!(commands.is_empty());
            }
            _ => panic!("Kỳ vọng Node 0 là SubGraph"),
        }
    }
}
