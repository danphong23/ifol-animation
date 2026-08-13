use std::collections::{HashMap, HashSet};
use std::ops::Range;
use thiserror::Error;
use crate::extensions::ExtensionId;
use crate::resources::handle::{BindGroupHandle, BufferHandle, ComputePipelineHandle, MeshHandle, PipelineHandle, RenderNodeId, TextureHandle};

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

    Indirect { buffer: BufferHandle, offset: u64 },
    IndexedIndirect { mesh: MeshHandle, buffer: BufferHandle, offset: u64 },
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeCommand {
    pub pipeline: ComputePipelineHandle,
    pub bind_groups: Vec<(u32, BindGroupHandle, Vec<u32>)>,
    pub workgroups: [u32; 3],
    pub indirect: Option<(BufferHandle, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyCommand {
    BufferToBuffer {
        source: BufferHandle,
        destination: BufferHandle,
        source_offset: u64,
        destination_offset: u64,
        size: u64,
    },
    TextureToTexture {
        source: TextureHandle,
        destination: TextureHandle,
        source_mip_level: u32,
        destination_mip_level: u32,
        source_origin: [u32; 3],
        destination_origin: [u32; 3],
        extent: [u32; 3],
    },
    TextureToTextureAspect {
        source: TextureHandle,
        destination: TextureHandle,
        source_mip_level: u32,
        destination_mip_level: u32,
        source_origin: [u32; 3],
        destination_origin: [u32; 3],
        extent: [u32; 3],
        aspect: TextureAspect,
    },
}

impl CopyCommand {
    pub fn buffer_to_buffer(source: BufferHandle, destination: BufferHandle, size: u64) -> Self {
        Self::BufferToBuffer { source, destination, source_offset: 0, destination_offset: 0, size }
    }

    pub fn with_offsets(mut self, source_offset: u64, destination_offset: u64) -> Self {
        if let Self::BufferToBuffer { source_offset: source, destination_offset: destination, .. } = &mut self {
            *source = source_offset;
            *destination = destination_offset;
        }
        self
    }

    pub fn texture_to_texture(source: TextureHandle, destination: TextureHandle, extent: [u32; 3]) -> Self {
        Self::TextureToTexture {
            source,
            destination,
            source_mip_level: 0,
            destination_mip_level: 0,
            source_origin: [0, 0, 0],
            destination_origin: [0, 0, 0],
            extent,
        }
    }

    pub fn texture_to_texture_aspect(
        source: TextureHandle,
        destination: TextureHandle,
        extent: [u32; 3],
        aspect: TextureAspect,
    ) -> Self {
        Self::TextureToTextureAspect {
            source,
            destination,
            source_mip_level: 0,
            destination_mip_level: 0,
            source_origin: [0, 0, 0],
            destination_origin: [0, 0, 0],
            extent,
            aspect,
        }
    }

    pub fn with_texture_mips(mut self, source_mip_level: u32, destination_mip_level: u32) -> Self {
        match &mut self {
            Self::TextureToTexture { source_mip_level: source, destination_mip_level: destination, .. }
            | Self::TextureToTextureAspect { source_mip_level: source, destination_mip_level: destination, .. } => {
            *source = source_mip_level;
            *destination = destination_mip_level;
            }
            _ => {}
        }
        self
    }
}

impl ComputeCommand {
    pub fn new(pipeline: ComputePipelineHandle, workgroups: [u32; 3]) -> Self {
        Self { pipeline, bind_groups: Vec::new(), workgroups, indirect: None }
    }

    pub fn new_indirect(pipeline: ComputePipelineHandle, buffer: BufferHandle, offset: u64) -> Self {
        Self { pipeline, bind_groups: Vec::new(), workgroups: [0; 3], indirect: Some((buffer, offset)) }
    }

    pub fn with_bind_group(mut self, slot: u32, handle: BindGroupHandle, offsets: Vec<u32>) -> Self {
        self.bind_groups.push((slot, handle, offsets));
        self
    }
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

    /// Render vào attachment multisample rồi resolve sang texture single-sample.
    OffscreenMsaa {
        color: TextureHandle,
        resolve: TextureHandle,
        width: u32,
        height: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphResource {
    Buffer(BufferHandle),
    Texture(TextureHandle),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceAccess {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureAspect {
    All,
    DepthOnly,
    StencilOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceSubresource {
    Whole,
    BufferRange { start: u64, end: u64 },
    Texture { mip_level: u32, array_layer: u32 },
    TextureRange { mip_start: u32, mip_end: u32, layer_start: u32, layer_end: u32 },
    TextureAspect { mip_level: u32, array_layer: u32, aspect: TextureAspect },
    TextureAspectRange { mip_start: u32, mip_end: u32, layer_start: u32, layer_end: u32, aspect: TextureAspect },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceUsage {
    pub resource: GraphResource,
    pub access: ResourceAccess,
    pub subresource: ResourceSubresource,
}

fn accesses_conflict(left: ResourceAccess, right: ResourceAccess) -> bool {
    !matches!((left, right), (ResourceAccess::Read, ResourceAccess::Read))
}

fn subresources_overlap(left: ResourceSubresource, right: ResourceSubresource) -> bool {
    match (left, right) {
        (ResourceSubresource::Whole, _) | (_, ResourceSubresource::Whole) => true,
        (ResourceSubresource::BufferRange { start: left_start, end: left_end }, ResourceSubresource::BufferRange { start: right_start, end: right_end }) => {
            left_start < right_end && right_start < left_end
        }
        (ResourceSubresource::BufferRange { .. }, _) | (_, ResourceSubresource::BufferRange { .. }) => false,
        (ResourceSubresource::Texture { mip_level: left_mip, array_layer: left_layer }, ResourceSubresource::TextureAspect { mip_level: right_mip, array_layer: right_layer, .. })
        | (ResourceSubresource::TextureAspect { mip_level: left_mip, array_layer: left_layer, .. }, ResourceSubresource::Texture { mip_level: right_mip, array_layer: right_layer }) => left_mip == right_mip && left_layer == right_layer,
        (ResourceSubresource::TextureRange { mip_start: left_mip_start, mip_end: left_mip_end, layer_start: left_layer_start, layer_end: left_layer_end }, ResourceSubresource::TextureAspectRange { mip_start: right_mip_start, mip_end: right_mip_end, layer_start: right_layer_start, layer_end: right_layer_end, .. })
        | (ResourceSubresource::TextureAspectRange { mip_start: left_mip_start, mip_end: left_mip_end, layer_start: left_layer_start, layer_end: left_layer_end, .. }, ResourceSubresource::TextureRange { mip_start: right_mip_start, mip_end: right_mip_end, layer_start: right_layer_start, layer_end: right_layer_end }) => left_mip_start < right_mip_end && right_mip_start < left_mip_end && left_layer_start < right_layer_end && right_layer_start < left_layer_end,
        (ResourceSubresource::TextureAspect { mip_level: left_mip, array_layer: left_layer, aspect: left_aspect }, ResourceSubresource::TextureAspect { mip_level: right_mip, array_layer: right_layer, aspect: right_aspect }) => left_mip == right_mip && left_layer == right_layer && aspects_overlap(left_aspect, right_aspect),
        (ResourceSubresource::TextureAspectRange { mip_start: left_mip_start, mip_end: left_mip_end, layer_start: left_layer_start, layer_end: left_layer_end, aspect: left_aspect }, ResourceSubresource::TextureAspectRange { mip_start: right_mip_start, mip_end: right_mip_end, layer_start: right_layer_start, layer_end: right_layer_end, aspect: right_aspect }) => left_mip_start < right_mip_end && right_mip_start < left_mip_end && left_layer_start < right_layer_end && right_layer_start < left_layer_end && aspects_overlap(left_aspect, right_aspect),
        (
            ResourceSubresource::Texture { mip_level: left_mip, array_layer: left_layer },
            ResourceSubresource::Texture { mip_level: right_mip, array_layer: right_layer },
        ) => left_mip == right_mip && left_layer == right_layer,
        (ResourceSubresource::Texture { mip_level, array_layer }, ResourceSubresource::TextureRange { mip_start, mip_end, layer_start, layer_end })
        | (ResourceSubresource::TextureRange { mip_start, mip_end, layer_start, layer_end }, ResourceSubresource::Texture { mip_level, array_layer }) => {
            mip_level >= mip_start && mip_level < mip_end && array_layer >= layer_start && array_layer < layer_end
        }
        (
            ResourceSubresource::TextureRange { mip_start: left_mip_start, mip_end: left_mip_end, layer_start: left_layer_start, layer_end: left_layer_end },
            ResourceSubresource::TextureRange { mip_start: right_mip_start, mip_end: right_mip_end, layer_start: right_layer_start, layer_end: right_layer_end },
        ) => left_mip_start < right_mip_end && right_mip_start < left_mip_end && left_layer_start < right_layer_end && right_layer_start < left_layer_end,
        (ResourceSubresource::TextureAspect { mip_level, array_layer, .. }, ResourceSubresource::TextureAspectRange { mip_start, mip_end, layer_start, layer_end, .. })
        | (ResourceSubresource::TextureAspectRange { mip_start, mip_end, layer_start, layer_end, .. }, ResourceSubresource::TextureAspect { mip_level, array_layer, .. }) => mip_level >= mip_start && mip_level < mip_end && array_layer >= layer_start && array_layer < layer_end,
        (ResourceSubresource::Texture { mip_level, array_layer }, ResourceSubresource::TextureAspectRange { mip_start, mip_end, layer_start, layer_end, .. })
        | (ResourceSubresource::TextureAspectRange { mip_start, mip_end, layer_start, layer_end, .. }, ResourceSubresource::Texture { mip_level, array_layer }) => mip_level >= mip_start && mip_level < mip_end && array_layer >= layer_start && array_layer < layer_end,
        _ => true,
    }
}

fn aspects_overlap(left: TextureAspect, right: TextureAspect) -> bool {
    matches!((left, right), (TextureAspect::All, _) | (_, TextureAspect::All)) || left == right
}

fn usages_conflict(left: &ResourceUsage, right: &ResourceUsage) -> bool {
    left.resource == right.resource
        && subresources_overlap(left.subresource, right.subresource)
        && accesses_conflict(left.access, right.access)
}

fn texture_subresource_range(mip_level: u32, origin: [u32; 3], extent: [u32; 3]) -> ResourceSubresource {
    let Some(layer_end) = origin[2].checked_add(extent[2]) else { return ResourceSubresource::Whole; };
    ResourceSubresource::TextureRange {
        mip_start: mip_level,
        mip_end: mip_level.saturating_add(1),
        layer_start: origin[2],
        layer_end,
    }
}

fn texture_aspect_subresource_range(
    mip_level: u32,
    origin: [u32; 3],
    extent: [u32; 3],
    aspect: TextureAspect,
) -> ResourceSubresource {
    let Some(layer_end) = origin[2].checked_add(extent[2]) else { return ResourceSubresource::Whole; };
    ResourceSubresource::TextureAspectRange {
        mip_start: mip_level,
        mip_end: mip_level.saturating_add(1),
        layer_start: origin[2],
        layer_end,
        aspect,
    }
}

fn buffer_subresource_range(offset: u64, size: u64) -> ResourceSubresource {
    if size == 0 { return ResourceSubresource::Whole; }
    let Some(end) = offset.checked_add(size) else { return ResourceSubresource::Whole; };
    ResourceSubresource::BufferRange { start: offset, end }
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
        bundle_key: Option<u64>,
    },

    /// Danh sách lệnh vẽ phẳng trên cùng 1 target.
    DrawBatch {
        commands: Vec<DrawCommand>,
        is_dirty: bool,
        use_bundle: bool,
        bundle: Option<wgpu::RenderBundle>,
        bundle_key: Option<u64>,
    },

    ComputeBatch {
        commands: Vec<ComputeCommand>,
        is_dirty: bool,
    },

    CopyBatch {
        commands: Vec<CopyCommand>,
    },

    /// Opaque host/extension operation ordered by declared resource usages.
    Extension {
        extension: ExtensionId,
        usages: Vec<ResourceUsage>,
    },
}

impl RenderNode {
    pub fn new_batch(commands: Vec<DrawCommand>) -> Self {
        Self::DrawBatch {
            commands,
            is_dirty: true,
            use_bundle: true, // Default to bundle enabled
            bundle: None,
            bundle_key: None,
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
            bundle_key: None,
        }
    }

    pub fn new_compute_batch(commands: Vec<ComputeCommand>) -> Self {
        Self::ComputeBatch { commands, is_dirty: true }
    }

    pub fn new_extension(extension: ExtensionId, usages: Vec<ResourceUsage>) -> Self {
        Self::Extension { extension, usages }
    }

    pub fn commands(&self) -> &[DrawCommand] {
        match self {
            Self::SubGraph { commands, .. } => commands,
            Self::DrawBatch { commands, .. } => commands,
            Self::ComputeBatch { .. } => &[],
            Self::CopyBatch { .. } => &[],
            Self::Extension { .. } => &[],
        }
    }

    pub fn compute_commands(&self) -> &[ComputeCommand] {
        match self {
            Self::ComputeBatch { commands, .. } => commands,
            _ => &[],
        }
    }

    pub fn copy_commands(&self) -> &[CopyCommand] {
        match self {
            Self::CopyBatch { commands } => commands,
            _ => &[],
        }
    }

    pub fn extension_usages(&self) -> &[ResourceUsage] {
        match self {
            Self::Extension { usages, .. } => usages,
            _ => &[],
        }
    }

    pub fn is_dirty(&self) -> bool {
        match self {
            Self::SubGraph { is_dirty, .. } => *is_dirty,
            Self::DrawBatch { is_dirty, .. } => *is_dirty,
            Self::ComputeBatch { is_dirty, .. } => *is_dirty,
            Self::CopyBatch { .. } => false,
            Self::Extension { .. } => false,
        }
    }

    pub fn bundle(&self) -> Option<&wgpu::RenderBundle> {
        match self {
            Self::SubGraph { bundle, .. } => bundle.as_ref(),
            Self::DrawBatch { bundle, .. } => bundle.as_ref(),
            Self::ComputeBatch { .. } => None,
            Self::CopyBatch { .. } => None,
            Self::Extension { .. } => None,
        }
    }

    pub fn bundle_key(&self) -> Option<u64> {
        match self {
            Self::SubGraph { bundle_key, .. } | Self::DrawBatch { bundle_key, .. } => *bundle_key,
            Self::ComputeBatch { .. } => None,
            Self::CopyBatch { .. } => None,
            Self::Extension { .. } => None,
        }
    }

    pub fn set_bundle_key(&mut self, key: u64) {
        match self {
            Self::SubGraph { bundle_key, .. } | Self::DrawBatch { bundle_key, .. } => *bundle_key = Some(key),
            Self::ComputeBatch { .. } => {},
            Self::CopyBatch { .. } => {},
            Self::Extension { .. } => {},
        }
    }

    pub fn set_use_bundle(&mut self, use_bundle: bool) {
        match self {
            Self::SubGraph { use_bundle: ub, is_dirty, .. } |
            Self::DrawBatch { use_bundle: ub, is_dirty, .. } => {
                *ub = use_bundle;
                *is_dirty = true;
            }
            Self::ComputeBatch { .. } => {},
            Self::CopyBatch { .. } => {},
            Self::Extension { .. } => {},
        }
    }

    pub fn use_bundle(&self) -> bool {
        match self {
            Self::SubGraph { use_bundle, .. } => *use_bundle,
            Self::DrawBatch { use_bundle, .. } => *use_bundle,
            Self::ComputeBatch { .. } => false,
            Self::CopyBatch { .. } => false,
            Self::Extension { .. } => false,
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
            Self::ComputeBatch { .. } => {},
            Self::CopyBatch { .. } => {},
            Self::Extension { .. } => {},
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

    pub fn alloc_compute_batch(&mut self, commands: Vec<ComputeCommand>) -> RenderNodeId {
        self.next_id += 1;
        let id = RenderNodeId(self.next_id);
        self.nodes.insert(id, RenderNode::new_compute_batch(commands));
        id
    }

    pub fn alloc_copy_batch(&mut self, commands: Vec<CopyCommand>) -> RenderNodeId {
        self.next_id += 1;
        let id = RenderNodeId(self.next_id);
        self.nodes.insert(id, RenderNode::CopyBatch { commands });
        id
    }

    pub fn alloc_extension(&mut self, extension: ExtensionId, usages: Vec<ResourceUsage>) -> RenderNodeId {
        self.next_id += 1;
        let id = RenderNodeId(self.next_id);
        self.nodes.insert(id, RenderNode::new_extension(extension, usages));
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
                RenderNode::DrawBatch { commands: cmds, is_dirty, bundle, bundle_key, .. } => {
                    *cmds = commands;
                    *is_dirty = true;
                    *bundle = None;
                    *bundle_key = None;
                }
                RenderNode::SubGraph { commands: cmds, is_dirty, bundle, bundle_key, .. } => {
                    *cmds = commands;
                    *is_dirty = true;
                    *bundle = None;
                    *bundle_key = None;
                }
                RenderNode::ComputeBatch { .. } => return false,
                RenderNode::CopyBatch { .. } => return false,
                RenderNode::Extension { .. } => return false,
            }
            true
        } else {
            false
        }
    }

    pub fn mark_dirty(&mut self, id: RenderNodeId) {
        if let Some(node) = self.nodes.get_mut(&id) {
            match node {
                RenderNode::DrawBatch { is_dirty, bundle, bundle_key, .. } |
                RenderNode::SubGraph { is_dirty, bundle, bundle_key, .. } => {
                    *is_dirty = true;
                    *bundle = None;
                    *bundle_key = None;
                }
                RenderNode::ComputeBatch { is_dirty, .. } => *is_dirty = true,
                RenderNode::CopyBatch { .. } => {},
                RenderNode::Extension { .. } => {},
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
    pub fn declare_resource_usage(&mut self, node: RenderNodeId, resource: GraphResource, access: ResourceAccess) {
        self.resource_usages.entry(node).or_default().push(ResourceUsage { resource, access, subresource: ResourceSubresource::Whole });
    }

    pub fn declare_texture_subresource_usage(
        &mut self,
        node: RenderNodeId,
        texture: TextureHandle,
        mip_level: u32,
        array_layer: u32,
        access: ResourceAccess,
    ) {
        self.resource_usages.entry(node).or_default().push(ResourceUsage {
            resource: GraphResource::Texture(texture),
            access,
            subresource: ResourceSubresource::Texture { mip_level, array_layer },
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
        self.resource_usages.entry(node).or_default().push(ResourceUsage {
            resource: GraphResource::Texture(texture),
            access,
            subresource: ResourceSubresource::TextureRange { mip_start, mip_end, layer_start, layer_end },
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
        self.resource_usages.entry(node).or_default().push(ResourceUsage {
            resource: GraphResource::Texture(texture),
            access,
            subresource: ResourceSubresource::TextureAspect { mip_level, array_layer, aspect },
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
        self.resource_usages.entry(node).or_default().push(ResourceUsage {
            resource: GraphResource::Texture(texture),
            access,
            subresource: ResourceSubresource::TextureAspectRange { mip_start, mip_end, layer_start, layer_end, aspect },
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
        self.resource_usages.entry(node).or_default().push(ResourceUsage {
            resource: GraphResource::Buffer(buffer),
            access,
            subresource: buffer_subresource_range(offset, size),
        });
    }

    pub fn resource_usages(&self, node: &RenderNodeId) -> &[ResourceUsage] {
        self.resource_usages.get(node).map(Vec::as_slice).unwrap_or(&[])
    }

    fn effective_resource_usages(&self, node_id: RenderNodeId, pool: &RenderNodePool) -> Vec<ResourceUsage> {
        let mut usages = self.resource_usages(&node_id).to_vec();
        if let Some(node) = pool.get(node_id) {
            usages.extend_from_slice(node.extension_usages());
            for command in node.copy_commands() {
                match command {
                    CopyCommand::BufferToBuffer { source, destination, source_offset, destination_offset, size } => {
                        usages.push(ResourceUsage { resource: GraphResource::Buffer(*source), access: ResourceAccess::Read, subresource: buffer_subresource_range(*source_offset, *size) });
                        usages.push(ResourceUsage { resource: GraphResource::Buffer(*destination), access: ResourceAccess::Write, subresource: buffer_subresource_range(*destination_offset, *size) });
                    }
                    CopyCommand::TextureToTexture { source, destination, source_mip_level, destination_mip_level, source_origin, destination_origin, extent } => {
                        let source_subresource = texture_subresource_range(*source_mip_level, *source_origin, *extent);
                        let destination_subresource = texture_subresource_range(*destination_mip_level, *destination_origin, *extent);
                        usages.push(ResourceUsage { resource: GraphResource::Texture(*source), access: ResourceAccess::Read, subresource: source_subresource });
                        usages.push(ResourceUsage { resource: GraphResource::Texture(*destination), access: ResourceAccess::Write, subresource: destination_subresource });
                    }
                    CopyCommand::TextureToTextureAspect { source, destination, source_mip_level, destination_mip_level, source_origin, destination_origin, extent, aspect } => {
                        let source_subresource = texture_aspect_subresource_range(*source_mip_level, *source_origin, *extent, *aspect);
                        let destination_subresource = texture_aspect_subresource_range(*destination_mip_level, *destination_origin, *extent, *aspect);
                        usages.push(ResourceUsage { resource: GraphResource::Texture(*source), access: ResourceAccess::Read, subresource: source_subresource });
                        usages.push(ResourceUsage { resource: GraphResource::Texture(*destination), access: ResourceAccess::Write, subresource: destination_subresource });
                    }
                }
            }
            for command in node.commands() {
                let indirect = match command.action {
                    DrawAction::Indirect { buffer, offset } => Some((buffer, offset, 16)),
                    DrawAction::IndexedIndirect { buffer, offset, .. } => Some((buffer, offset, 20)),
                    _ => None,
                };
                if let Some((buffer, offset, size)) = indirect {
                    usages.push(ResourceUsage { resource: GraphResource::Buffer(buffer), access: ResourceAccess::Read, subresource: buffer_subresource_range(offset, size) });
                }
            }
            for command in node.compute_commands() {
                if let Some((buffer, offset)) = command.indirect {
                    usages.push(ResourceUsage { resource: GraphResource::Buffer(buffer), access: ResourceAccess::Read, subresource: buffer_subresource_range(offset, 12) });
                }
            }
            if !node.commands().is_empty() {
                match self.target {
                    RenderTarget::Offscreen { color, .. } => {
                        usages.push(ResourceUsage { resource: GraphResource::Texture(color), access: ResourceAccess::Write, subresource: ResourceSubresource::Whole });
                    }
                    RenderTarget::OffscreenMsaa { color, resolve, .. } => {
                        usages.push(ResourceUsage { resource: GraphResource::Texture(color), access: ResourceAccess::Write, subresource: ResourceSubresource::Whole });
                        usages.push(ResourceUsage { resource: GraphResource::Texture(resolve), access: ResourceAccess::Write, subresource: ResourceSubresource::Whole });
                    }
                    RenderTarget::Screen => {}
                }
                if let Some(depth) = self.depth_stencil {
                    usages.push(ResourceUsage { resource: GraphResource::Texture(depth), access: ResourceAccess::Write, subresource: ResourceSubresource::Whole });
                }
            }
        }
        usages
    }

    /// Trả về thứ tự của các node trực tiếp thuộc graph sau khi áp dụng
    /// dependency. Declaration order là tie-breaker ổn định.
    pub fn ordered_node_ids(&self, pool: &RenderNodePool) -> Result<Vec<RenderNodeId>, GraphFlattenError> {
        let positions = self.node_ids.iter().enumerate().map(|(index, &id)| (id, index)).collect::<HashMap<_, _>>();
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
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(dependency.before));
            };
            let Some(&after) = positions.get(&dependency.after) else {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(dependency.after));
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
                    after_usages.iter().any(|right| usages_conflict(left, right))
                });
                if conflict {
                    add_edge(before, after);
                }
            }
        }

        let mut ordered = Vec::with_capacity(self.node_ids.len());
        let mut emitted = vec![false; self.node_ids.len()];
        while ordered.len() < self.node_ids.len() {
            let Some(index) = (0..self.node_ids.len()).find(|&index| !emitted[index] && indegree[index] == 0) else {
                let cycle = self.node_ids.iter().enumerate().find(|(index, _)| !emitted[*index]).map(|(_, &id)| id).unwrap_or(RenderNodeId(0));
                return Err(GraphFlattenError::Cycle(cycle));
            };
            emitted[index] = true;
            ordered.push(self.node_ids[index]);
            for &next in &edges[index] { indegree[next] -= 1; }
        }
        Ok(ordered)
    }

    pub fn add_batch(&mut self, pool: &mut RenderNodePool, commands: Vec<DrawCommand>) -> RenderNodeId {
        let id = pool.alloc_batch(commands);
        self.node_ids.push(id);
        id
    }

    pub fn add_compute_batch(&mut self, pool: &mut RenderNodePool, commands: Vec<ComputeCommand>) -> RenderNodeId {
        let id = pool.alloc_compute_batch(commands);
        self.node_ids.push(id);
        id
    }

    pub fn add_copy_batch(&mut self, pool: &mut RenderNodePool, commands: Vec<CopyCommand>) -> RenderNodeId {
        let id = pool.alloc_copy_batch(commands);
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
        let positions = plan.nodes.iter().enumerate().map(|(index, node)| (node.node_id, index)).collect::<HashMap<_, _>>();
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
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(dependency.before));
            };
            let Some(&after) = positions.get(&dependency.after) else {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(dependency.after));
            };
            add_edge(before, after);
        }

        for before in 0..plan.nodes.len() {
            for after in (before + 1)..plan.nodes.len() {
                let before_usages = usage_map.get(&plan.nodes[before].node_id).map(Vec::as_slice).unwrap_or(&[]);
                let after_usages = usage_map.get(&plan.nodes[after].node_id).map(Vec::as_slice).unwrap_or(&[]);
                if before_usages.iter().any(|left| after_usages.iter().any(|right| usages_conflict(left, right))) {
                    add_edge(before, after);
                }
            }
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

    fn collect_dependencies(
        &self,
        pool: &RenderNodePool,
        dependencies: &mut Vec<GraphDependency>,
    ) -> Result<(), GraphFlattenError> {
        let node_set: HashSet<_> = self.node_ids.iter().copied().collect();
        for dependency in &self.dependencies {
            if !node_set.contains(&dependency.before) {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(dependency.before));
            }
            if !node_set.contains(&dependency.after) {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(dependency.after));
            }
            dependencies.push(*dependency);
        }
        for &node_id in &self.node_ids {
            let node = pool.get(node_id).ok_or(GraphFlattenError::MissingNode(node_id))?;
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
            let node = pool.get(node_id).ok_or(GraphFlattenError::MissingNode(node_id))?;
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
    fn flatten_keeps_extension_node_and_uses_its_resource_hazards() {
        let mut pool = RenderNodePool::new();
        let extension = pool.alloc_extension(
            crate::extensions::ExtensionId::new("test.filter").unwrap(),
            vec![ResourceUsage {
                resource: GraphResource::Texture(TextureHandle(9)),
                access: ResourceAccess::Write,
                subresource: ResourceSubresource::Whole,
            }],
        );
        let reader = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(extension);
        graph.add_node_id(reader);
        graph.declare_resource_usage(reader, GraphResource::Texture(TextureHandle(9)), ResourceAccess::Read);

        let plan = graph.flatten(&pool).unwrap();
        assert_eq!(plan.nodes.iter().map(|node| node.node_id).collect::<Vec<_>>(), vec![extension, reader]);
        assert_eq!(pool.get(extension).unwrap().extension_usages().len(), 1);
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

    #[test]
    fn direct_execution_order_uses_explicit_dependency() {
        let mut pool = RenderNodePool::new();
        let first = pool.alloc_batch(vec![]);
        let second = pool.alloc_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(first);
        graph.add_node_id(second);
        graph.add_dependency(second, first);

        assert_eq!(graph.ordered_node_ids(&pool).unwrap(), vec![second, first]);
    }

    #[test]
    fn resource_write_then_read_creates_implicit_hazard_edge() {
        let mut pool = RenderNodePool::new();
        let writer = pool.alloc_copy_batch(vec![]);
        let reader = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(writer);
        graph.add_node_id(reader);
        graph.declare_resource_usage(writer, GraphResource::Buffer(BufferHandle(1)), ResourceAccess::Write);
        graph.declare_resource_usage(reader, GraphResource::Buffer(BufferHandle(1)), ResourceAccess::Read);

        assert_eq!(graph.ordered_node_ids(&pool).unwrap(), vec![writer, reader]);
    }

    #[test]
    fn copy_commands_infer_source_read_and_destination_write_hazard() {
        let mut pool = RenderNodePool::new();
        let copy = pool.alloc_copy_batch(vec![CopyCommand::buffer_to_buffer(BufferHandle(1), BufferHandle(2), 4)]);
        let later_read = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(copy);
        graph.add_node_id(later_read);
        graph.declare_resource_usage(later_read, GraphResource::Buffer(BufferHandle(2)), ResourceAccess::Read);

        assert_eq!(graph.ordered_node_ids(&pool).unwrap(), vec![copy, later_read]);
    }

    #[test]
    fn texture_copy_hazard_uses_mip_and_layer_range() {
        let mut pool = RenderNodePool::new();
        let copy = pool.alloc_copy_batch(vec![CopyCommand::TextureToTexture {
            source: TextureHandle(1), destination: TextureHandle(2),
            source_mip_level: 0, destination_mip_level: 0,
            source_origin: [0, 0, 0], destination_origin: [0, 0, 0], extent: [4, 4, 2],
        }]);
        let later_writer = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(copy);
        graph.add_node_id(later_writer);
        graph.declare_texture_subresource_usage(later_writer, TextureHandle(1), 1, 0, ResourceAccess::Write);
        graph.add_dependency(later_writer, copy);

        assert_eq!(graph.ordered_node_ids(&pool).unwrap(), vec![later_writer, copy]);
    }

    #[test]
    fn buffer_copy_hazard_uses_byte_range() {
        let mut pool = RenderNodePool::new();
        let copy = pool.alloc_copy_batch(vec![CopyCommand::BufferToBuffer {
            source: BufferHandle(1), destination: BufferHandle(2),
            source_offset: 0, destination_offset: 0, size: 16,
        }]);
        let later_writer = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(copy);
        graph.add_node_id(later_writer);
        graph.declare_buffer_range_usage(later_writer, BufferHandle(1), 32, 16, ResourceAccess::Write);
        graph.add_dependency(later_writer, copy);

        assert_eq!(graph.ordered_node_ids(&pool).unwrap(), vec![later_writer, copy]);
    }

    #[test]
    fn disjoint_depth_and_stencil_aspects_do_not_create_hazard_edge() {
        let mut pool = RenderNodePool::new();
        let depth_writer = pool.alloc_compute_batch(vec![]);
        let stencil_writer = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(depth_writer);
        graph.add_node_id(stencil_writer);
        graph.declare_texture_aspect_usage(depth_writer, TextureHandle(9), 0, 0, TextureAspect::DepthOnly, ResourceAccess::Write);
        graph.declare_texture_aspect_usage(stencil_writer, TextureHandle(9), 0, 0, TextureAspect::StencilOnly, ResourceAccess::Write);
        graph.add_dependency(stencil_writer, depth_writer);

        assert_eq!(graph.ordered_node_ids(&pool).unwrap(), vec![stencil_writer, depth_writer]);
    }

    #[test]
    fn all_texture_aspect_overlaps_depth_and_stencil() {
        assert!(aspects_overlap(TextureAspect::All, TextureAspect::DepthOnly));
        assert!(aspects_overlap(TextureAspect::StencilOnly, TextureAspect::All));
        assert!(!aspects_overlap(TextureAspect::DepthOnly, TextureAspect::StencilOnly));
    }

    #[test]
    fn explicit_reverse_dependency_conflicts_with_hazard_edge() {
        let mut pool = RenderNodePool::new();
        let writer = pool.alloc_copy_batch(vec![]);
        let reader = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(writer);
        graph.add_node_id(reader);
        graph.declare_resource_usage(writer, GraphResource::Buffer(BufferHandle(2)), ResourceAccess::Write);
        graph.declare_resource_usage(reader, GraphResource::Buffer(BufferHandle(2)), ResourceAccess::Read);
        graph.add_dependency(reader, writer);

        assert!(matches!(graph.ordered_node_ids(&pool), Err(GraphFlattenError::Cycle(_))));
    }

    #[test]
    fn disjoint_texture_subresources_do_not_create_hazard_edge() {
        let mut pool = RenderNodePool::new();
        let writer = pool.alloc_copy_batch(vec![]);
        let reader = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(writer);
        graph.add_node_id(reader);
        graph.declare_texture_subresource_usage(writer, TextureHandle(7), 0, 0, ResourceAccess::Write);
        graph.declare_texture_subresource_usage(reader, TextureHandle(7), 1, 0, ResourceAccess::Read);
        graph.add_dependency(reader, writer);

        assert_eq!(graph.ordered_node_ids(&pool).unwrap(), vec![reader, writer]);
    }

    #[test]
    fn overlapping_texture_subresources_create_hazard_edge() {
        let mut pool = RenderNodePool::new();
        let writer = pool.alloc_copy_batch(vec![]);
        let reader = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(writer);
        graph.add_node_id(reader);
        graph.declare_texture_subresource_usage(writer, TextureHandle(7), 0, 0, ResourceAccess::Write);
        graph.declare_texture_subresource_usage(reader, TextureHandle(7), 0, 0, ResourceAccess::Read);
        graph.add_dependency(reader, writer);

        assert!(matches!(graph.ordered_node_ids(&pool), Err(GraphFlattenError::Cycle(_))));
    }

    #[test]
    fn flatten_applies_hazard_between_nested_and_root_nodes() {
        let mut pool = RenderNodePool::new();
        let nested_writer = pool.alloc_copy_batch(vec![]);
        let mut child = RenderGraph::new(RenderTarget::Screen);
        child.add_node_id(nested_writer);
        child.declare_resource_usage(nested_writer, GraphResource::Texture(TextureHandle(10)), ResourceAccess::Write);
        let subgraph = pool.alloc_subgraph("producer", child, vec![]);
        let reader = pool.alloc_compute_batch(vec![]);
        let mut root = RenderGraph::new(RenderTarget::Screen);
        root.add_node_id(subgraph);
        root.add_node_id(reader);
        root.declare_resource_usage(reader, GraphResource::Texture(TextureHandle(10)), ResourceAccess::Read);

        let plan = root.flatten(&pool).unwrap();
        assert_eq!(plan.nodes.iter().map(|node| node.node_id).collect::<Vec<_>>(), vec![nested_writer, subgraph, reader]);
    }

    #[test]
    fn flatten_applies_explicit_dependency_inside_nested_graph() {
        let mut pool = RenderNodePool::new();
        let first = pool.alloc_batch(vec![]);
        let second = pool.alloc_batch(vec![]);
        let mut child = RenderGraph::new(RenderTarget::Screen);
        child.add_node_id(first);
        child.add_node_id(second);
        child.add_dependency(second, first);
        let subgraph = pool.alloc_subgraph("ordered_child", child, vec![]);
        let mut root = RenderGraph::new(RenderTarget::Screen);
        root.add_node_id(subgraph);

        let plan = root.flatten(&pool).unwrap();
        assert_eq!(plan.nodes.iter().map(|node| node.node_id).collect::<Vec<_>>(), vec![second, first, subgraph]);
    }
}
