use super::usage::{
    buffer_subresource_range, texture_aspect_subresource_range, texture_subresource_range,
    usages_conflict,
};
use super::{
    ComputeCommand, CopyCommand, DrawAction, DrawCommand, GraphResource, RenderNode,
    RenderNodePool, RenderTarget, ResourceAccess, ResourceSubresource, ResourceUsage,
    TextureAspect,
};
use crate::resources::handle::{BufferHandle, RenderNodeId, TextureHandle};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

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
    resource_usages: HashMap<RenderNodeId, Vec<ResourceUsage>>,
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
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
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

    /// Khai báo resource mà node đọc/ghi. Đây là metadata cho hazard compiler;
    /// command encoder hiện tại vẫn giữ behavior cũ nếu graph không khai báo.
    pub fn declare_resource_usage(
        &mut self,
        node: RenderNodeId,
        resource: GraphResource,
        access: ResourceAccess,
    ) {
        self.resource_usages
            .entry(node)
            .or_default()
            .push(ResourceUsage {
                resource,
                access,
                subresource: ResourceSubresource::Whole,
            });
    }

    pub fn declare_texture_subresource_usage(
        &mut self,
        node: RenderNodeId,
        texture: TextureHandle,
        mip_level: u32,
        array_layer: u32,
        access: ResourceAccess,
    ) {
        self.resource_usages
            .entry(node)
            .or_default()
            .push(ResourceUsage {
                resource: GraphResource::Texture(texture),
                access,
                subresource: ResourceSubresource::Texture {
                    mip_level,
                    array_layer,
                },
            });
    }

    pub fn declare_texture_subresource_range_usage(
        &mut self,
        node: RenderNodeId,
        texture: TextureHandle,
        mip_start: u32,
        mip_end: u32,
        layer_start: u32,
        layer_end: u32,
        access: ResourceAccess,
    ) {
        self.resource_usages
            .entry(node)
            .or_default()
            .push(ResourceUsage {
                resource: GraphResource::Texture(texture),
                access,
                subresource: ResourceSubresource::TextureRange {
                    mip_start,
                    mip_end,
                    layer_start,
                    layer_end,
                },
            });
    }

    pub fn declare_texture_aspect_usage(
        &mut self,
        node: RenderNodeId,
        texture: TextureHandle,
        mip_level: u32,
        array_layer: u32,
        aspect: TextureAspect,
        access: ResourceAccess,
    ) {
        self.resource_usages
            .entry(node)
            .or_default()
            .push(ResourceUsage {
                resource: GraphResource::Texture(texture),
                access,
                subresource: ResourceSubresource::TextureAspect {
                    mip_level,
                    array_layer,
                    aspect,
                },
            });
    }

    pub fn declare_texture_aspect_range_usage(
        &mut self,
        node: RenderNodeId,
        texture: TextureHandle,
        mip_start: u32,
        mip_end: u32,
        layer_start: u32,
        layer_end: u32,
        aspect: TextureAspect,
        access: ResourceAccess,
    ) {
        self.resource_usages
            .entry(node)
            .or_default()
            .push(ResourceUsage {
                resource: GraphResource::Texture(texture),
                access,
                subresource: ResourceSubresource::TextureAspectRange {
                    mip_start,
                    mip_end,
                    layer_start,
                    layer_end,
                    aspect,
                },
            });
    }

    pub fn declare_buffer_range_usage(
        &mut self,
        node: RenderNodeId,
        buffer: BufferHandle,
        offset: u64,
        size: u64,
        access: ResourceAccess,
    ) {
        self.resource_usages
            .entry(node)
            .or_default()
            .push(ResourceUsage {
                resource: GraphResource::Buffer(buffer),
                access,
                subresource: buffer_subresource_range(offset, size),
            });
    }

    pub fn resource_usages(&self, node: &RenderNodeId) -> &[ResourceUsage] {
        self.resource_usages
            .get(node)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn effective_resource_usages(
        &self,
        node_id: RenderNodeId,
        pool: &RenderNodePool,
    ) -> Vec<ResourceUsage> {
        let mut usages = self.resource_usages(&node_id).to_vec();
        if let Some(node) = pool.get(node_id) {
            usages.extend_from_slice(node.extension_usages());
            for command in node.copy_commands() {
                match command {
                    CopyCommand::BufferToBuffer {
                        source,
                        destination,
                        source_offset,
                        destination_offset,
                        size,
                    } => {
                        usages.push(ResourceUsage {
                            resource: GraphResource::Buffer(*source),
                            access: ResourceAccess::Read,
                            subresource: buffer_subresource_range(*source_offset, *size),
                        });
                        usages.push(ResourceUsage {
                            resource: GraphResource::Buffer(*destination),
                            access: ResourceAccess::Write,
                            subresource: buffer_subresource_range(*destination_offset, *size),
                        });
                    }
                    CopyCommand::TextureToTexture {
                        source,
                        destination,
                        source_mip_level,
                        destination_mip_level,
                        source_origin,
                        destination_origin,
                        extent,
                    } => {
                        let source_subresource =
                            texture_subresource_range(*source_mip_level, *source_origin, *extent);
                        let destination_subresource = texture_subresource_range(
                            *destination_mip_level,
                            *destination_origin,
                            *extent,
                        );
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(*source),
                            access: ResourceAccess::Read,
                            subresource: source_subresource,
                        });
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(*destination),
                            access: ResourceAccess::Write,
                            subresource: destination_subresource,
                        });
                    }
                    CopyCommand::TextureToTextureAspect {
                        source,
                        destination,
                        source_mip_level,
                        destination_mip_level,
                        source_origin,
                        destination_origin,
                        extent,
                        aspect,
                    } => {
                        let source_subresource = texture_aspect_subresource_range(
                            *source_mip_level,
                            *source_origin,
                            *extent,
                            *aspect,
                        );
                        let destination_subresource = texture_aspect_subresource_range(
                            *destination_mip_level,
                            *destination_origin,
                            *extent,
                            *aspect,
                        );
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(*source),
                            access: ResourceAccess::Read,
                            subresource: source_subresource,
                        });
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(*destination),
                            access: ResourceAccess::Write,
                            subresource: destination_subresource,
                        });
                    }
                }
            }
            for command in node.commands() {
                let indirect = match command.action {
                    DrawAction::Indirect { buffer, offset } => Some((buffer, offset, 16)),
                    DrawAction::IndexedIndirect { buffer, offset, .. } => {
                        Some((buffer, offset, 20))
                    }
                    _ => None,
                };
                if let Some((buffer, offset, size)) = indirect {
                    usages.push(ResourceUsage {
                        resource: GraphResource::Buffer(buffer),
                        access: ResourceAccess::Read,
                        subresource: buffer_subresource_range(offset, size),
                    });
                }
            }
            for command in node.compute_commands() {
                if let Some((buffer, offset)) = command.indirect {
                    usages.push(ResourceUsage {
                        resource: GraphResource::Buffer(buffer),
                        access: ResourceAccess::Read,
                        subresource: buffer_subresource_range(offset, 12),
                    });
                }
            }
            if !node.commands().is_empty() {
                match self.target {
                    RenderTarget::Offscreen { color, .. } => {
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(color),
                            access: ResourceAccess::Write,
                            subresource: ResourceSubresource::Whole,
                        });
                    }
                    RenderTarget::OffscreenMsaa { color, resolve, .. } => {
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(color),
                            access: ResourceAccess::Write,
                            subresource: ResourceSubresource::Whole,
                        });
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(resolve),
                            access: ResourceAccess::Write,
                            subresource: ResourceSubresource::Whole,
                        });
                    }
                    RenderTarget::Screen => {}
                }
                if let Some(depth) = self.depth_stencil {
                    usages.push(ResourceUsage {
                        resource: GraphResource::Texture(depth),
                        access: ResourceAccess::Write,
                        subresource: ResourceSubresource::Whole,
                    });
                }
            }
        }
        usages
    }

    /// Trả về thứ tự của các node trực tiếp thuộc graph sau khi áp dụng
    /// dependency. Declaration order là tie-breaker ổn định.
    pub fn ordered_node_ids(
        &self,
        pool: &RenderNodePool,
    ) -> Result<Vec<RenderNodeId>, GraphFlattenError> {
        let positions = self
            .node_ids
            .iter()
            .enumerate()
            .map(|(index, &id)| (id, index))
            .collect::<HashMap<_, _>>();
        for &node_id in &self.node_ids {
            if pool.get(node_id).is_none() {
                return Err(GraphFlattenError::MissingNode(node_id));
            }
        }
        let mut edges = vec![Vec::new(); self.node_ids.len()];
        let mut indegree = vec![0usize; self.node_ids.len()];
        let mut edge_set = HashSet::new();
        let mut add_edge = |before: usize, after: usize| {
            if edge_set.insert((before, after)) {
                edges[before].push(after);
                indegree[after] += 1;
            }
        };
        for dependency in &self.dependencies {
            let Some(&before) = positions.get(&dependency.before) else {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(
                    dependency.before,
                ));
            };
            let Some(&after) = positions.get(&dependency.after) else {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(
                    dependency.after,
                ));
            };
            add_edge(before, after);
        }

        // Declaration order is the stable tie-breaker for implicit hazards.
        // A later node cannot observe/write the same resource before the earlier
        // node when at least one side writes it.
        for before in 0..self.node_ids.len() {
            for after in (before + 1)..self.node_ids.len() {
                let before_usages = self.effective_resource_usages(self.node_ids[before], pool);
                let after_usages = self.effective_resource_usages(self.node_ids[after], pool);
                let conflict = before_usages.iter().any(|left| {
                    after_usages
                        .iter()
                        .any(|right| usages_conflict(left, right))
                });
                if conflict {
                    add_edge(before, after);
                }
            }
        }

        let mut ordered = Vec::with_capacity(self.node_ids.len());
        let mut emitted = vec![false; self.node_ids.len()];
        while ordered.len() < self.node_ids.len() {
            let Some(index) =
                (0..self.node_ids.len()).find(|&index| !emitted[index] && indegree[index] == 0)
            else {
                let cycle = self
                    .node_ids
                    .iter()
                    .enumerate()
                    .find(|(index, _)| !emitted[*index])
                    .map(|(_, &id)| id)
                    .unwrap_or(RenderNodeId(0));
                return Err(GraphFlattenError::Cycle(cycle));
            };
            emitted[index] = true;
            ordered.push(self.node_ids[index]);
            for &next in &edges[index] {
                indegree[next] -= 1;
            }
        }
        Ok(ordered)
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

    /// Làm phẳng logical graph theo thứ tự thực thi bottom-up: node con của
    /// `SubGraph` xuất hiện trước node composite của chính subgraph.
    pub fn flatten(&self, pool: &RenderNodePool) -> Result<FlatRenderPlan, GraphFlattenError> {
        let mut plan = FlatRenderPlan::default();
        let mut active = Vec::new();
        let mut usage_map = HashMap::new();
        self.flatten_into(pool, &mut plan, &mut active, Vec::new(), &mut usage_map)?;
        let mut dependencies = Vec::new();
        self.collect_dependencies(pool, &mut dependencies)?;
        Self::apply_dependencies(&mut plan, &usage_map, &dependencies)?;
        Ok(plan)
    }

    fn apply_dependencies(
        plan: &mut FlatRenderPlan,
        usage_map: &HashMap<RenderNodeId, Vec<ResourceUsage>>,
        dependencies: &[GraphDependency],
    ) -> Result<(), GraphFlattenError> {
        if plan.nodes.len() < 2 {
            return Ok(());
        }
        let positions = plan
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.node_id, index))
            .collect::<HashMap<_, _>>();
        let mut edges = vec![Vec::new(); plan.nodes.len()];
        let mut indegree = vec![0usize; plan.nodes.len()];
        let mut edge_set = HashSet::new();
        let mut add_edge = |before: usize, after: usize| {
            if edge_set.insert((before, after)) {
                edges[before].push(after);
                indegree[after] += 1;
            }
        };
        for dependency in dependencies {
            let Some(&before) = positions.get(&dependency.before) else {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(
                    dependency.before,
                ));
            };
            let Some(&after) = positions.get(&dependency.after) else {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(
                    dependency.after,
                ));
            };
            add_edge(before, after);
        }

        for before in 0..plan.nodes.len() {
            for after in (before + 1)..plan.nodes.len() {
                let before_usages = usage_map
                    .get(&plan.nodes[before].node_id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let after_usages = usage_map
                    .get(&plan.nodes[after].node_id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                if before_usages.iter().any(|left| {
                    after_usages
                        .iter()
                        .any(|right| usages_conflict(left, right))
                }) {
                    add_edge(before, after);
                }
            }
        }

        let original = plan.nodes.clone();
        let mut ordered = Vec::with_capacity(original.len());
        let mut emitted = vec![false; original.len()];
        while ordered.len() < original.len() {
            let Some(index) =
                (0..original.len()).find(|&index| !emitted[index] && indegree[index] == 0)
            else {
                let cycle = original
                    .iter()
                    .find(|node| !emitted[positions[&node.node_id]])
                    .map(|node| node.node_id)
                    .unwrap_or(RenderNodeId(0));
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

    fn collect_dependencies(
        &self,
        pool: &RenderNodePool,
        dependencies: &mut Vec<GraphDependency>,
    ) -> Result<(), GraphFlattenError> {
        let node_set: HashSet<_> = self.node_ids.iter().copied().collect();
        for dependency in &self.dependencies {
            if !node_set.contains(&dependency.before) {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(
                    dependency.before,
                ));
            }
            if !node_set.contains(&dependency.after) {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(
                    dependency.after,
                ));
            }
            dependencies.push(*dependency);
        }
        for &node_id in &self.node_ids {
            let node = pool
                .get(node_id)
                .ok_or(GraphFlattenError::MissingNode(node_id))?;
            if let RenderNode::SubGraph { graph, .. } = node {
                graph.collect_dependencies(pool, dependencies)?;
            }
        }
        Ok(())
    }

    fn flatten_into(
        &self,
        pool: &RenderNodePool,
        plan: &mut FlatRenderPlan,
        active: &mut Vec<RenderNodeId>,
        parent_path: Vec<RenderNodeId>,
        usage_map: &mut HashMap<RenderNodeId, Vec<ResourceUsage>>,
    ) -> Result<(), GraphFlattenError> {
        for &node_id in &self.node_ids {
            if active.contains(&node_id) {
                return Err(GraphFlattenError::Cycle(node_id));
            }
            let node = pool
                .get(node_id)
                .ok_or(GraphFlattenError::MissingNode(node_id))?;
            usage_map.insert(node_id, self.effective_resource_usages(node_id, pool));
            let mut path = parent_path.clone();
            path.push(node_id);
            if let RenderNode::SubGraph { graph, .. } = node {
                active.push(node_id);
                graph.flatten_into(pool, plan, active, path.clone(), usage_map)?;
                active.pop();
            }
            plan.nodes.push(FlatRenderNode { node_id, path });
        }
        Ok(())
    }
}
