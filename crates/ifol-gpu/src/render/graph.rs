use std::collections::HashMap;
use std::ops::Range;
use thiserror::Error;
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
        use_bundle: bool,
        bundle: Option<wgpu::RenderBundle>,
    },

    /// Danh sách lệnh vẽ phẳng trên cùng 1 target.
    DrawBatch {
        commands: Vec<DrawCommand>,
        is_dirty: bool,
        use_bundle: bool,
        bundle: Option<wgpu::RenderBundle>,
    },
}

impl RenderNode {
    pub fn new_batch(commands: Vec<DrawCommand>) -> Self {
        Self::DrawBatch {
            commands,
            is_dirty: true,
            use_bundle: true, // Default to bundle enabled
            bundle: None,
        }
    }

    pub fn new_subgraph(name: impl Into<String>, graph: RenderGraph, commands: Vec<DrawCommand>) -> Self {
        Self::SubGraph {
            name: name.into(),
            graph: Box::new(graph),
            commands,
            is_dirty: true,
            use_bundle: true,
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

    pub fn set_use_bundle(&mut self, use_bundle: bool) {
        match self {
            Self::SubGraph { use_bundle: ub, is_dirty, .. } |
            Self::DrawBatch { use_bundle: ub, is_dirty, .. } => {
                *ub = use_bundle;
                *is_dirty = true;
            }
        }
    }

    pub fn use_bundle(&self) -> bool {
        match self {
            Self::SubGraph { use_bundle, .. } => *use_bundle,
            Self::DrawBatch { use_bundle, .. } => *use_bundle,
        }
    }

    /// Tự động sắp xếp các DrawCommand theo Pipeline -> BindGroup
    /// Giúp giảm State Thrashing khi GPU chạy
    pub fn sort_by_state(&mut self) {
        match self {
            Self::SubGraph { commands, is_dirty, .. } |
            Self::DrawBatch { commands, is_dirty, .. } => {
                commands.sort_by(|a, b| {
                    // Sort by Pipeline first
                    let cmp = a.pipeline.0.cmp(&b.pipeline.0);
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                    // Sort by the first bind group handle if it exists
                    let bg_a = a.bind_groups.first().map(|bg| bg.1 .0).unwrap_or(0);
                    let bg_b = b.bind_groups.first().map(|bg| bg.1 .0).unwrap_or(0);
                    bg_a.cmp(&bg_b)
                });
                *is_dirty = true;
            }
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
                RenderNode::DrawBatch { commands: cmds, is_dirty, bundle, .. } => {
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

    /// Duyệt từ cuối lên đầu thay vì từ đầu tới cuối (Reverse Draw Order)
    pub reverse_draw_order: bool,
    pub dependencies: Vec<GraphDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatRenderNode {
    pub node_id: RenderNodeId,
    /// Chuỗi node từ root tới node này, dùng cho diagnostics/profiling.
    pub path: Vec<RenderNodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FlatRenderPlan {
    pub nodes: Vec<FlatRenderNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphDependency {
    pub before: RenderNodeId,
    pub after: RenderNodeId,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GraphFlattenError {
    #[error("render node {0:?} does not exist in the node pool")]
    MissingNode(RenderNodeId),
    #[error("cycle detected while flattening render graph at node {0:?}")]
    Cycle(RenderNodeId),
    #[error("dependency references node {0:?} outside the graph")]
    DependencyNodeOutsideGraph(RenderNodeId),
}

impl FlatRenderPlan {
    pub fn len(&self) -> usize { self.nodes.len() }
    pub fn is_empty(&self) -> bool { self.nodes.is_empty() }
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

    /// Làm phẳng logical graph theo thứ tự thực thi bottom-up: node con của
    /// `SubGraph` xuất hiện trước node composite của chính subgraph.
    pub fn flatten(&self, pool: &RenderNodePool) -> Result<FlatRenderPlan, GraphFlattenError> {
        let mut plan = FlatRenderPlan::default();
        let mut active = Vec::new();
        self.flatten_into(pool, &mut plan, &mut active, Vec::new())?;
        self.apply_dependencies(&mut plan)?;
        Ok(plan)
    }

    fn apply_dependencies(&self, plan: &mut FlatRenderPlan) -> Result<(), GraphFlattenError> {
        if self.dependencies.is_empty() || plan.nodes.len() < 2 {
            return Ok(());
        }
        let positions = plan.nodes.iter().enumerate().map(|(index, node)| (node.node_id, index)).collect::<HashMap<_, _>>();
        let mut edges = vec![Vec::new(); plan.nodes.len()];
        let mut indegree = vec![0usize; plan.nodes.len()];
        for dependency in &self.dependencies {
            let Some(&before) = positions.get(&dependency.before) else {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(dependency.before));
            };
            let Some(&after) = positions.get(&dependency.after) else {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(dependency.after));
            };
            edges[before].push(after);
            indegree[after] += 1;
        }

        let original = plan.nodes.clone();
        let mut ordered = Vec::with_capacity(original.len());
        let mut emitted = vec![false; original.len()];
        while ordered.len() < original.len() {
            let Some(index) = (0..original.len()).find(|&index| !emitted[index] && indegree[index] == 0) else {
                let cycle = original.iter().find(|node| !emitted[positions[&node.node_id]]).map(|node| node.node_id).unwrap_or(RenderNodeId(0));
                return Err(GraphFlattenError::Cycle(cycle));
            };
            emitted[index] = true;
            ordered.push(original[index].clone());
            for &next in &edges[index] {
                indegree[next] -= 1;
            }
        }
        plan.nodes = ordered;
        Ok(())
    }

    fn flatten_into(
        &self,
        pool: &RenderNodePool,
        plan: &mut FlatRenderPlan,
        active: &mut Vec<RenderNodeId>,
        parent_path: Vec<RenderNodeId>,
    ) -> Result<(), GraphFlattenError> {
        for &node_id in &self.node_ids {
            if active.contains(&node_id) {
                return Err(GraphFlattenError::Cycle(node_id));
            }
            let node = pool.get(node_id).ok_or(GraphFlattenError::MissingNode(node_id))?;
            let mut path = parent_path.clone();
            path.push(node_id);
            if let RenderNode::SubGraph { graph, .. } = node {
                active.push(node_id);
                graph.flatten_into(pool, plan, active, path.clone())?;
                active.pop();
            }
            plan.nodes.push(FlatRenderNode { node_id, path });
        }
        Ok(())
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

    #[test]
    fn flatten_orders_child_nodes_before_subgraph_composite() {
        let mut pool = RenderNodePool::new();
        let child_batch = pool.alloc_batch(vec![]);
        let mut child_graph = RenderGraph::new(RenderTarget::Screen);
        child_graph.add_node_id(child_batch);
        let subgraph = pool.alloc_subgraph("child", child_graph, vec![]);
        let root_batch = pool.alloc_batch(vec![]);
        let mut root = RenderGraph::new(RenderTarget::Screen);
        root.add_node_id(subgraph);
        root.add_node_id(root_batch);

        let plan = root.flatten(&pool).unwrap();

        assert_eq!(plan.nodes.iter().map(|node| node.node_id).collect::<Vec<_>>(), vec![child_batch, subgraph, root_batch]);
        assert_eq!(plan.nodes[0].path, vec![subgraph, child_batch]);
    }

    #[test]
    fn flatten_reports_missing_node() {
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(RenderNodeId(99));

        assert_eq!(
            graph.flatten(&RenderNodePool::new()),
            Err(GraphFlattenError::MissingNode(RenderNodeId(99)))
        );
    }

    #[test]
    fn flatten_applies_explicit_dependency_with_declaration_order_tiebreaker() {
        let mut pool = RenderNodePool::new();
        let first = pool.alloc_batch(vec![]);
        let second = pool.alloc_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(first);
        graph.add_node_id(second);
        graph.add_dependency(second, first);

        let plan = graph.flatten(&pool).unwrap();
        assert_eq!(plan.nodes.iter().map(|node| node.node_id).collect::<Vec<_>>(), vec![second, first]);
    }

    #[test]
    fn flatten_rejects_dependency_cycle() {
        let mut pool = RenderNodePool::new();
        let first = pool.alloc_batch(vec![]);
        let second = pool.alloc_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(first);
        graph.add_node_id(second);
        graph.add_dependency(first, second);
        graph.add_dependency(second, first);

        assert!(matches!(graph.flatten(&pool), Err(GraphFlattenError::Cycle(_))));
    }
}
