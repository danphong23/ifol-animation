use thiserror::Error;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use crate::api::{GpuEngine, ProfilingError, TimestampQueryPool, TimestampSpan};
use crate::extensions::{ExtensionDispatchRegistry, ExtensionExecutionContext};
use crate::memory::{SubmissionId, SubmissionTracker};
use crate::graph::{CopyCommand, DrawAction, GraphResource, RenderGraph, RenderNode, RenderNodePool, RenderTarget};
use crate::resources::handle::{BindGroupHandle, BufferHandle, ComputePipelineHandle, MeshHandle, PipelineHandle, RenderNodeId, TextureHandle};
use crate::resources::registry::ResourceRegistry;

pub struct RenderGraphExecutor {
    context_key: u64,
    extension_dispatchers: Arc<ExtensionDispatchRegistry>,
}

/// Thống kê cấu trúc của một lần thực thi graph.
///
/// Đây là diagnostics hook cấp core, không giả vờ là GPU timing. Host có thể
/// dùng report để log, kiểm thử regression hoặc nối vào profiler riêng.
#[derive(Debug, Clone)]
pub struct ExecutionReport {
    pub submission: wgpu::SubmissionIndex,
    pub flattened_nodes: usize,
    pub draw_commands: usize,
    pub compute_commands: usize,
    pub copy_commands: usize,
    pub indirect_commands: usize,
    pub declared_usages: usize,
}

#[derive(Debug, Clone)]
pub struct ProfiledExecution {
    pub report: ExecutionReport,
    pub span: TimestampSpan,
    /// Submission identity used by the optional host-side completion tracker.
    /// `None` means the untracked profiling API was used.
    pub tracking_submission: Option<SubmissionId>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RenderGraphProfilingError {
    #[error(transparent)]
    Validation(#[from] RenderGraphValidationError),
    #[error(transparent)]
    Profiling(#[from] ProfilingError),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RenderGraphValidationError {
    #[error("extension operation {0:?} has no executor dispatch registered")]
    UnsupportedExtension(crate::extensions::ExtensionId),
    #[error("extension operation {extension:?} failed validation: {error}")]
    ExtensionValidation {
        extension: crate::extensions::ExtensionId,
        error: crate::extensions::ExtensionValidationError,
    },
    #[error("extension operation {extension:?} failed during dispatch: {error}")]
    ExtensionDispatch {
        extension: crate::extensions::ExtensionId,
        error: crate::extensions::ExtensionExecutionError,
    },
    #[error("render node {0:?} does not exist in the node pool")]
    MissingNode(RenderNodeId),
    #[error("render graph dependency cycle involves node {0:?}")]
    DependencyCycle(RenderNodeId),
    #[error("render graph dependency references node {0:?} outside the graph")]
    DependencyOutsideGraph(RenderNodeId),
    #[error("texture resource {0:?} is missing")]
    MissingTexture(TextureHandle),
    #[error("pipeline resource {0:?} is missing")]
    MissingPipeline(PipelineHandle),
    #[error("compute pipeline resource {0:?} is missing")]
    MissingComputePipeline(ComputePipelineHandle),
    #[error("buffer resource {0:?} is missing")]
    MissingBuffer(BufferHandle),
    #[error("declared resource usage references missing buffer {0:?}")]
    MissingUsageBuffer(BufferHandle),
    #[error("declared resource usage references missing texture {0:?}")]
    MissingUsageTexture(TextureHandle),
    #[error("buffer {handle:?} is missing required usage bits {required_usage:#x}; actual {actual_usage:#x}")]
    MissingBufferUsage { handle: BufferHandle, required_usage: u32, actual_usage: u32 },
    #[error("owned texture resource {0:?} is required for texture copy")]
    MissingOwnedTexture(TextureHandle),
    #[error("texture resource {0:?} has no descriptor metadata")]
    MissingTextureDescriptor(TextureHandle),
    #[error("texture copy formats differ: source {source_handle:?}, destination {destination_handle:?}")]
    TextureCopyFormatMismatch { source_handle: TextureHandle, destination_handle: TextureHandle },
    #[error("texture copy extent must be non-zero, got {extent:?}")]
    InvalidTextureCopyExtent { extent: [u32; 3] },
    #[error("texture {handle:?} does not support copy aspect {aspect:?}")]
    InvalidTextureAspect { handle: TextureHandle, aspect: crate::graph::TextureAspect },
    #[error("texture copy mip level {mip_level} is invalid for {handle:?} (mip count {mip_count})")]
    InvalidTextureMipLevel { handle: TextureHandle, mip_level: u32, mip_count: u32 },
    #[error("texture copy range for {handle:?} exceeds mip extent {mip_extent:?}: origin {origin:?}, extent {extent:?}")]
    InvalidTextureCopyRange { handle: TextureHandle, origin: [u32; 3], extent: [u32; 3], mip_extent: [u32; 3] },
    #[error("copy range for buffer {handle:?} exceeds buffer size: offset {offset}, size {size}, buffer size {buffer_size}")]
    InvalidCopyRange { handle: BufferHandle, offset: u64, size: u64, buffer_size: u64 },
    #[error("mesh resource {0:?} is missing")]
    MissingMesh(MeshHandle),
    #[error("bind group resource {0:?} is missing")]
    MissingBindGroup(BindGroupHandle),
    #[error("indirect buffer {0:?} is missing")]
    MissingIndirectBuffer(BufferHandle),
    #[error("indirect buffer {handle:?} is missing required usage bits {required_usage:#x}; actual {actual_usage:#x}")]
    MissingIndirectBufferUsage { handle: BufferHandle, required_usage: u32, actual_usage: u32 },
    #[error("indirect buffer {handle:?} range is invalid: offset {offset}, size {size}")]
    InvalidIndirectRange { handle: BufferHandle, offset: u64, size: u64 },
    #[error("indexed indirect draw requires mesh {0:?} to have an index buffer")]
    MissingIndexBuffer(MeshHandle),
    #[error("bind group slot {slot} is outside the device limit {max_slots}")]
    InvalidBindGroupSlot { slot: u32, max_slots: u32 },
    #[error("bind group {handle:?} expects {expected} dynamic offsets, got {actual}")]
    InvalidDynamicOffsetCount { handle: BindGroupHandle, expected: u32, actual: u32 },
    #[error("dynamic offset {offset} for bind group {handle:?} is not aligned to {alignment}")]
    InvalidDynamicOffsetAlignment { handle: BindGroupHandle, offset: u32, alignment: u32 },
    #[error("pipeline {pipeline:?} has no bind-group layout metadata for bind group {bind_group:?} at slot {slot}")]
    MissingPipelineLayoutMetadata { pipeline: PipelineHandle, bind_group: BindGroupHandle, slot: u32 },
    #[error("compute pipeline {pipeline:?} has no bind-group layout metadata for bind group {bind_group:?} at slot {slot}")]
    MissingComputePipelineLayoutMetadata { pipeline: ComputePipelineHandle, bind_group: BindGroupHandle, slot: u32 },
    #[error("pipeline {pipeline:?} layout mismatch at slot {slot}: expected {expected:?}, actual {actual:?}")]
    PipelineLayoutMismatch { pipeline: PipelineHandle, slot: u32, expected: Option<u64>, actual: Option<u64> },
    #[error("compute pipeline {pipeline:?} layout mismatch at slot {slot}: expected {expected:?}, actual {actual:?}")]
    ComputePipelineLayoutMismatch { pipeline: ComputePipelineHandle, slot: u32, expected: Option<u64>, actual: Option<u64> },
    #[error("render target dimensions must be non-zero, got {width}x{height}")]
    InvalidTargetSize { width: u32, height: u32 },
    #[error("texture {handle:?} has descriptor size {actual_width}x{actual_height}, graph requested {width}x{height}")]
    TargetSizeMismatch {
        handle: TextureHandle,
        width: u32,
        height: u32,
        actual_width: u32,
        actual_height: u32,
    },
    #[error("texture {handle:?} is missing required usage bits {required_usage:#x}; actual {actual_usage:#x}")]
    MissingTextureUsage {
        handle: TextureHandle,
        required_usage: u32,
        actual_usage: u32,
    },
    #[error("texture {handle:?} uses sample count {actual}, but this render path supports only sample count 1")]
    UnsupportedSampleCount { handle: TextureHandle, actual: u32 },
    #[error("MSAA resolve texture {0:?} is missing")]
    MissingResolveTexture(TextureHandle),
    #[error("MSAA resolve texture {handle:?} must be single-sample, got {actual}")]
    InvalidResolveSampleCount { handle: TextureHandle, actual: u32 },
    #[error("MSAA color and resolve formats differ: color {color:?}, resolve {resolve:?}")]
    ResolveFormatMismatch { color: wgpu::TextureFormat, resolve: wgpu::TextureFormat },
    #[error("MSAA color and resolve dimensions differ: color {color_width}x{color_height}, resolve {resolve_width}x{resolve_height}")]
    ResolveSizeMismatch { color_width: u32, color_height: u32, resolve_width: u32, resolve_height: u32 },
    #[error("depth texture {handle:?} sample count mismatch: expected {expected}, got {actual}")]
    DepthSampleCountMismatch { handle: TextureHandle, expected: u32, actual: u32 },
}

fn bind_group_slot_index(slot: u32, max_slots: u32) -> Option<usize> {
    (slot < max_slots).then_some(slot as usize)
}

fn validate_bind_group_offsets(
    registry: &ResourceRegistry,
    handle: BindGroupHandle,
    offsets: &[u32],
) -> Result<(), RenderGraphValidationError> {
    let Some(descriptor) = registry.bind_group_descriptor(&handle) else { return Ok(()); };
    if offsets.len() as u32 != descriptor.dynamic_offset_count {
        return Err(RenderGraphValidationError::InvalidDynamicOffsetCount {
            handle,
            expected: descriptor.dynamic_offset_count,
            actual: offsets.len() as u32,
        });
    }
    for &offset in offsets {
        if offset % descriptor.dynamic_offset_alignment != 0 {
            return Err(RenderGraphValidationError::InvalidDynamicOffsetAlignment {
                handle,
                offset,
                alignment: descriptor.dynamic_offset_alignment,
            });
        }
    }
    Ok(())
}

fn validate_render_pipeline_layout(
    registry: &ResourceRegistry,
    pipeline: PipelineHandle,
    slot: u32,
    bind_group: BindGroupHandle,
) -> Result<(), RenderGraphValidationError> {
    let Some(descriptor) = registry.pipeline_layout_descriptor(&pipeline) else { return Ok(()); };
    let expected = descriptor.bind_group_layout_signatures.get(slot as usize).copied().flatten();
    let actual = registry.bind_group_descriptor(&bind_group).map(|descriptor| descriptor.layout_signature);
    if expected.is_some() && actual.is_none() {
        return Err(RenderGraphValidationError::MissingPipelineLayoutMetadata { pipeline, bind_group, slot });
    }
    if expected != actual {
        return Err(RenderGraphValidationError::PipelineLayoutMismatch { pipeline, slot, expected, actual });
    }
    Ok(())
}

fn validate_compute_pipeline_layout(
    registry: &ResourceRegistry,
    pipeline: ComputePipelineHandle,
    slot: u32,
    bind_group: BindGroupHandle,
) -> Result<(), RenderGraphValidationError> {
    let Some(descriptor) = registry.compute_pipeline_layout_descriptor(&pipeline) else { return Ok(()); };
    let expected = descriptor.bind_group_layout_signatures.get(slot as usize).copied().flatten();
    let actual = registry.bind_group_descriptor(&bind_group).map(|descriptor| descriptor.layout_signature);
    if expected.is_some() && actual.is_none() {
        return Err(RenderGraphValidationError::MissingComputePipelineLayoutMetadata { pipeline, bind_group, slot });
    }
    if expected != actual {
        return Err(RenderGraphValidationError::ComputePipelineLayoutMismatch { pipeline, slot, expected, actual });
    }
    Ok(())
}

fn format_has_stencil(format: wgpu::TextureFormat) -> bool {
    matches!(
        format,
        wgpu::TextureFormat::Stencil8
            | wgpu::TextureFormat::Depth24PlusStencil8
            | wgpu::TextureFormat::Depth32FloatStencil8
    )
}

fn bundle_cache_key(
    node: &RenderNode,
    registry: &ResourceRegistry,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    sample_count: u32,
    context_key: u64,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    color_format.hash(&mut hasher);
    depth_format.hash(&mut hasher);
    sample_count.hash(&mut hasher);
    context_key.hash(&mut hasher);
    for command in node.commands() {
        command.pipeline.0.hash(&mut hasher);
        registry.pipeline_version(&command.pipeline).hash(&mut hasher);
        for &(slot, bind_group, ref offsets) in &command.bind_groups {
            slot.hash(&mut hasher);
            bind_group.0.hash(&mut hasher);
            registry.bind_group_version(&bind_group).hash(&mut hasher);
            offsets.hash(&mut hasher);
        }
        if let DrawAction::Indexed { mesh, .. } = command.action {
            mesh.0.hash(&mut hasher);
            registry.mesh_version(&mesh).hash(&mut hasher);
        }
    }
    hasher.finish()
}

impl Default for RenderGraphExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderGraphExecutor {
    pub fn new() -> Self {
        Self { context_key: 0, extension_dispatchers: Arc::new(ExtensionDispatchRegistry::new()) }
    }

    /// Gán identity ổn định cho device/viewport mà host đang dùng. Hai context
    /// khác nhau không được dùng chung bundle dù logical node giống nhau.
    pub fn with_context_key(context_key: u64) -> Self {
        Self { context_key, ..Self::new() }
    }

    pub fn with_extension_dispatchers(dispatchers: ExtensionDispatchRegistry) -> Self {
        Self { context_key: 0, extension_dispatchers: Arc::new(dispatchers) }
    }

    pub fn with_context_and_extension_dispatchers(
        context_key: u64,
        dispatchers: ExtensionDispatchRegistry,
    ) -> Self {
        Self { context_key, extension_dispatchers: Arc::new(dispatchers) }
    }

    pub fn context_key(&self) -> u64 {
        self.context_key
    }

    fn dispatch_extension(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &RenderNodePool,
        node_id: RenderNodeId,
    ) -> Result<(), RenderGraphValidationError> {
        let Some(RenderNode::Extension { extension, usages }) = pool.get(node_id) else {
            return Ok(());
        };
        let Some(dispatcher) = self.extension_dispatchers.get(extension) else {
            return Err(RenderGraphValidationError::UnsupportedExtension(extension.clone()));
        };
        dispatcher
            .encode(ExtensionExecutionContext::new(engine, registry, encoder, node_id, usages))
            .map_err(|error| RenderGraphValidationError::ExtensionDispatch {
                extension: extension.clone(),
                error,
            })
    }

    /// Kiểm tra graph trước khi tạo command buffer. Đây là API được khuyến nghị
    /// cho host muốn nhận lỗi typed thay vì behavior silent-skip của prototype.
    pub fn validate(
        &self,
        registry: &ResourceRegistry,
        pool: &RenderNodePool,
        graph: &RenderGraph,
    ) -> Result<(), RenderGraphValidationError> {
        validate_graph(registry, pool, graph, wgpu::Limits::default().max_bind_groups, &self.extension_dispatchers)
    }

    /// Validate graph theo capability của device mà host thực sự sẽ dùng.
    /// Dùng API này khi host cần chẩn đoán trước execute mà vẫn giữ đúng
    /// `max_bind_groups` của adapter.
    pub fn validate_with_device(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &RenderNodePool,
        graph: &RenderGraph,
    ) -> Result<(), RenderGraphValidationError> {
        validate_graph(registry, pool, graph, engine.capabilities().max_bind_groups, &self.extension_dispatchers)
    }

    pub fn execute_checked(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
    ) -> Result<wgpu::SubmissionIndex, RenderGraphValidationError> {
        Ok(self.execute_checked_with_report(engine, registry, pool, graph)?.submission)
    }

    pub fn execute_checked_with_report(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
    ) -> Result<ExecutionReport, RenderGraphValidationError> {
        self.execute_with_surface_checked_with_report(engine, registry, pool, graph, None)
    }

    pub fn execute_with_surface_checked(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        surface_view: Option<&wgpu::TextureView>,
    ) -> Result<wgpu::SubmissionIndex, RenderGraphValidationError> {
        Ok(self.execute_with_surface_checked_with_report(engine, registry, pool, graph, surface_view)?.submission)
    }

    pub fn execute_with_surface_checked_with_report(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        surface_view: Option<&wgpu::TextureView>,
    ) -> Result<ExecutionReport, RenderGraphValidationError> {
        self.validate_with_device(engine, registry, pool, graph)?;
        let (flattened_nodes, draw_commands, compute_commands, copy_commands, indirect_commands, declared_usages) =
            Self::execution_counts_for_graph(pool, graph)?;
        let submission = self.execute_unchecked(engine, registry, pool, graph, surface_view)?;
        Ok(ExecutionReport {
            submission,
            flattened_nodes,
            draw_commands,
            compute_commands,
            copy_commands,
            indirect_commands,
            declared_usages,
        })
    }

    /// Thực thi graph và ghi một span timestamp bao quanh toàn bộ flat/compile
    /// boundary. Đây là API opt-in; graph thông thường không chịu overhead này.
    pub fn execute_checked_with_timestamp(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        profiler: &mut TimestampQueryPool,
        resolve_buffer: &wgpu::Buffer,
        resolve_offset: u64,
    ) -> Result<ProfiledExecution, RenderGraphProfilingError> {
        self.execute_timestamped(
            engine, registry, pool, graph, None, profiler, resolve_buffer, resolve_offset, None,
        )
    }

    /// Biên dịch và submit graph có profiling, đồng thời đăng ký query pool với
    /// `SubmissionTracker` trước khi submit. Host vẫn chịu trách nhiệm gọi
    /// `mark_completed` khi GPU hoàn tất submission.
    pub fn execute_checked_with_timestamp_tracked(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        profiler: &mut TimestampQueryPool,
        resolve_buffer: &wgpu::Buffer,
        resolve_offset: u64,
        tracker: &mut SubmissionTracker,
    ) -> Result<ProfiledExecution, RenderGraphProfilingError> {
        self.execute_timestamped(
            engine, registry, pool, graph, None, profiler, resolve_buffer, resolve_offset, Some(tracker),
        )
    }

    pub fn execute_with_surface_checked_with_timestamp(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        surface_view: Option<&wgpu::TextureView>,
        profiler: &mut TimestampQueryPool,
        resolve_buffer: &wgpu::Buffer,
        resolve_offset: u64,
    ) -> Result<ProfiledExecution, RenderGraphProfilingError> {
        self.execute_timestamped(
            engine, registry, pool, graph, surface_view, profiler, resolve_buffer, resolve_offset, None,
        )
    }

    /// Bản profiling có surface và lifecycle submission được tracker quản lý.
    pub fn execute_with_surface_checked_with_timestamp_tracked(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        surface_view: Option<&wgpu::TextureView>,
        profiler: &mut TimestampQueryPool,
        resolve_buffer: &wgpu::Buffer,
        resolve_offset: u64,
        tracker: &mut SubmissionTracker,
    ) -> Result<ProfiledExecution, RenderGraphProfilingError> {
        self.execute_timestamped(
            engine, registry, pool, graph, surface_view, profiler, resolve_buffer, resolve_offset, Some(tracker),
        )
    }

    fn execute_timestamped(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        surface_view: Option<&wgpu::TextureView>,
        profiler: &mut TimestampQueryPool,
        resolve_buffer: &wgpu::Buffer,
        resolve_offset: u64,
        mut tracker: Option<&mut SubmissionTracker>,
    ) -> Result<ProfiledExecution, RenderGraphProfilingError> {
        self.validate_with_device(engine, registry, pool, graph)?;
        let (flattened_nodes, draw_commands, compute_commands, copy_commands, indirect_commands, declared_usages) =
            Self::execution_counts_for_graph(pool, graph)?;
        let span = profiler.allocate_span()?;
        let mut encoder = engine.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("RenderGraphProfiledEncoder"),
        });
        profiler.write_span(&mut encoder, span)?;
        self.compile_flat_graph(&mut encoder, engine, pool, graph, registry, surface_view)?;
        profiler.write_span(&mut encoder, span)?;
        profiler.resolve_span(&mut encoder, span, resolve_buffer, resolve_offset)?;
        let tracking_submission = if let Some(tracker) = tracker.as_deref_mut() {
            let submission = tracker.begin();
            profiler.mark_submitted(submission)?;
            Some(submission)
        } else {
            None
        };
        let submission = engine.queue().submit(std::iter::once(encoder.finish()));
        Ok(ProfiledExecution {
            report: ExecutionReport {
                submission,
                flattened_nodes,
                draw_commands,
                compute_commands,
                copy_commands,
                indirect_commands,
                declared_usages,
            },
            span,
            tracking_submission,
        })
    }

    /// Biên dịch RenderGraph thành các lệnh gọi WGPU và đẩy xuống GPU Queue.
    pub fn execute(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
    ) -> Result<wgpu::SubmissionIndex, RenderGraphValidationError> {
        self.execute_checked(engine, registry, pool, graph)
    }

    /// Biên dịch RenderGraph với Surface Texture View chỉ định (khi vẽ trực tiếp ra cửa sổ)
    pub fn execute_with_surface(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        surface_view: Option<&wgpu::TextureView>,
    ) -> Result<wgpu::SubmissionIndex, RenderGraphValidationError> {
        self.execute_with_surface_checked(engine, registry, pool, graph, surface_view)
    }

    fn execute_unchecked(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        surface_view: Option<&wgpu::TextureView>,
    ) -> Result<wgpu::SubmissionIndex, RenderGraphValidationError> {
        let mut encoder = engine.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("RenderGraphEncoder"),
        });

        // Duyệt 2-Phase cây RenderGraph
        self.compile_flat_graph(&mut encoder, engine, pool, graph, registry, surface_view)?;

        // Submit toàn bộ khối lệnh (Command Buffer) lên GPU 1 lần duy nhất
        Ok(engine.queue().submit(std::iter::once(encoder.finish())))
}

fn execution_counts_for_graph(
    pool: &RenderNodePool,
    graph: &RenderGraph,
) -> Result<(usize, usize, usize, usize, usize, usize), RenderGraphValidationError> {
    let plan = graph.flatten(pool).map_err(|error| match error {
        crate::graph::GraphFlattenError::MissingNode(node) => RenderGraphValidationError::MissingNode(node),
        crate::graph::GraphFlattenError::Cycle(node) => RenderGraphValidationError::DependencyCycle(node),
        crate::graph::GraphFlattenError::DependencyNodeOutsideGraph(node) => RenderGraphValidationError::DependencyOutsideGraph(node),
    })?;
    let mut draws = 0;
    let mut computes = 0;
    let mut copies = 0;
    let mut indirect = 0;
    let usages = Self::declared_usage_count(pool, graph);
    for flat_node in &plan.nodes {
        let Some(node) = pool.get(flat_node.node_id) else { continue; };
        draws += node.commands().len();
        computes += node.compute_commands().len();
        copies += node.copy_commands().len();
        indirect += node.commands().iter().filter(|command| matches!(command.action, DrawAction::Indirect { .. } | DrawAction::IndexedIndirect { .. })).count();
        indirect += node.compute_commands().iter().filter(|command| command.indirect.is_some()).count();
    }
    Ok((plan.nodes.len(), draws, computes, copies, indirect, usages))
}

fn map_graph_flatten_error(error: crate::graph::GraphFlattenError) -> RenderGraphValidationError {
    match error {
        crate::graph::GraphFlattenError::MissingNode(node) => RenderGraphValidationError::MissingNode(node),
        crate::graph::GraphFlattenError::Cycle(node) => RenderGraphValidationError::DependencyCycle(node),
        crate::graph::GraphFlattenError::DependencyNodeOutsideGraph(node) => RenderGraphValidationError::DependencyOutsideGraph(node),
    }
}

fn declared_usage_count(pool: &RenderNodePool, graph: &RenderGraph) -> usize {
    graph.node_ids.iter().fold(0, |count, node_id| {
        let nested = match pool.get(*node_id) {
            Some(RenderNode::SubGraph { graph: child, .. }) => Self::declared_usage_count(pool, child),
            _ => 0,
        };
        let extension_usage_count = pool.get(*node_id).map_or(0, |node| node.extension_usages().len());
        count + graph.resource_usages(node_id).len() + extension_usage_count + nested
    })
}

fn execute_non_render_nodes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        engine: &GpuEngine,
        pool: &RenderNodePool,
        registry: &ResourceRegistry,
        node_ids: &[RenderNodeId],
    ) -> Result<(), RenderGraphValidationError> {
        for &node_id in node_ids {
            let Some(node) = pool.get(node_id) else { continue; };
            self.dispatch_extension(encoder, engine, registry, pool, node_id)?;
            for command in node.copy_commands() {
                encode_copy_command(encoder, registry, command);
            }
            if !node.compute_commands().is_empty() {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("RenderGraphComputePass"), timestamp_writes: None,
                });
                let mut current_pipeline = None;
                let mut current_bind_groups = vec![None; engine.capabilities().max_bind_groups as usize];
                for command in node.compute_commands() {
                    if current_pipeline != Some(command.pipeline) {
                        if let Some(pipeline) = registry.compute_pipeline(&command.pipeline) {
                            compute_pass.set_pipeline(pipeline);
                            current_pipeline = Some(command.pipeline);
                        } else { continue; }
                    }
                    for &(slot, bind_group, ref offsets) in &command.bind_groups {
                        let Some(slot_index) = bind_group_slot_index(slot, engine.capabilities().max_bind_groups) else { continue; };
                        if current_bind_groups[slot_index] != Some(bind_group) || !offsets.is_empty() {
                            if let Some(group) = registry.bind_group(&bind_group) {
                                compute_pass.set_bind_group(slot, group, offsets);
                                current_bind_groups[slot_index] = Some(bind_group);
                            }
                        }
                    }
                    if let Some((buffer, offset)) = command.indirect {
                        if let Some(indirect) = registry.buffer(&buffer) { compute_pass.dispatch_workgroups_indirect(indirect, offset); }
                    } else {
                        compute_pass.dispatch_workgroups(command.workgroups[0], command.workgroups[1], command.workgroups[2]);
                    }
                }
            }
        }
        Ok(())
    }

    fn execute_ordered_target_nodes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        engine: &GpuEngine,
        pool: &RenderNodePool,
        registry: &ResourceRegistry,
        node_ids: &[RenderNodeId],
        color_view: &wgpu::TextureView,
        color_format: wgpu::TextureFormat,
        resolve_view: Option<&wgpu::TextureView>,
        depth_stencil_info: Option<(&wgpu::TextureView, wgpu::TextureFormat)>,
        clear_color: Option<[f32; 4]>,
        max_bind_groups: u32,
    ) -> Result<(), RenderGraphValidationError> {
        let mut rendered_any = false;
        for &node_id in node_ids {
            let Some(node) = pool.get(node_id) else { continue; };
            self.dispatch_extension(encoder, engine, registry, pool, node_id)?;
            for command in node.copy_commands() {
                encode_copy_command(encoder, registry, command);
            }
            encode_compute_commands(encoder, registry, node.compute_commands(), max_bind_groups);
            if node.commands().is_empty() { continue; }

            let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                depth_slice: None,
                resolve_target: resolve_view,
                ops: wgpu::Operations {
                    load: if !rendered_any {
                        clear_color.map(|c| wgpu::LoadOp::Clear(wgpu::Color { r: c[0] as f64, g: c[1] as f64, b: c[2] as f64, a: c[3] as f64 })).unwrap_or(wgpu::LoadOp::Load)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                },
            })];
            let depth_stencil_attachment = depth_stencil_info.map(|(view, format)| wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: if !rendered_any && clear_color.is_some() { wgpu::LoadOp::Clear(1.0) } else { wgpu::LoadOp::Load },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: format_has_stencil(format).then_some(wgpu::Operations {
                    load: if !rendered_any && clear_color.is_some() { wgpu::LoadOp::Clear(0) } else { wgpu::LoadOp::Load },
                    store: wgpu::StoreOp::Store,
                }),
            });
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderGraphSegmentPass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            encode_draw_commands(&mut render_pass, registry, node.commands(), max_bind_groups);
            drop(render_pass);
            rendered_any = true;
        }
        let _ = (color_format, rendered_any);
        Ok(())
    }

    fn owner_graph_for_flat_path<'a>(
        root: &'a RenderGraph,
        pool: &'a RenderNodePool,
        path: &[RenderNodeId],
    ) -> Result<&'a RenderGraph, RenderGraphValidationError> {
        let mut owner = root;
        for &ancestor_id in path.iter().take(path.len().saturating_sub(1)) {
            let Some(RenderNode::SubGraph { graph, .. }) = pool.get(ancestor_id) else {
                return Err(RenderGraphValidationError::MissingNode(ancestor_id));
            };
            owner = graph;
        }
        Ok(owner)
    }

    fn flat_plan_owner_path(node: &crate::graph::FlatRenderNode) -> Vec<RenderNodeId> {
        node.path[..node.path.len().saturating_sub(1)].to_vec()
    }

    /// Encode the flattened logical plan in exactly the order produced by
    /// `RenderGraph::flatten`. Each node is encoded against the graph that
    /// owns it; this is what allows a root node and a nested node to be
    /// reordered by an explicit dependency or an inferred hazard.
    fn compile_flat_graph(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        engine: &GpuEngine,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        registry: &ResourceRegistry,
        surface_view: Option<&wgpu::TextureView>,
    ) -> Result<(), RenderGraphValidationError> {
        let plan = graph.flatten(pool).map_err(Self::map_graph_flatten_error)?;
        let is_direct_plan = plan.nodes.len() == graph.node_ids.len()
            && plan.nodes.iter().zip(&graph.node_ids).all(|(flat, direct)| flat.node_id == *direct);
        if is_direct_plan {
            return self.compile_graph(encoder, engine, pool, graph, registry, surface_view);
        }
        let mut last_draw_index = HashMap::<Vec<RenderNodeId>, usize>::new();
        for (index, flat_node) in plan.nodes.iter().enumerate() {
            if pool.get(flat_node.node_id).is_some_and(|node| !node.commands().is_empty()) {
                last_draw_index.insert(Self::flat_plan_owner_path(flat_node), index);
            }
        }
        let mut rendered_targets = HashSet::<Vec<RenderNodeId>>::new();

        for (index, flat_node) in plan.nodes.iter().enumerate() {
            let Some(node) = pool.get(flat_node.node_id) else {
                return Err(RenderGraphValidationError::MissingNode(flat_node.node_id));
            };
            let owner_path = Self::flat_plan_owner_path(flat_node);
            let owner = Self::owner_graph_for_flat_path(graph, pool, &flat_node.path)?;

            self.dispatch_extension(encoder, engine, registry, pool, flat_node.node_id)?;
            for command in node.copy_commands() {
                encode_copy_command(encoder, registry, command);
            }
            encode_compute_commands(
                encoder,
                registry,
                node.compute_commands(),
                engine.capabilities().max_bind_groups,
            );
            if node.commands().is_empty() {
                continue;
            }

            let target_view_info = match &owner.target {
                RenderTarget::Screen => surface_view
                    .zip(engine.surface_format())
                    .map(|(view, format)| (view, format, 1, None))
                    .or_else(|| registry.texture(&TextureHandle(0)).map(|(view, format)| (view, *format, 1, None))),
                RenderTarget::Offscreen { color, .. } => registry.texture(color).map(|(view, format)| (view, *format, 1, None)),
                RenderTarget::OffscreenMsaa { color, resolve, .. } => registry.texture(color).and_then(|(color_view, format)| {
                    registry.texture(resolve).map(|(resolve_view, _)| {
                        (
                            color_view,
                            *format,
                            registry.texture_descriptor(color).map_or(1, |descriptor| descriptor.sample_count),
                            Some(resolve_view),
                        )
                    })
                }),
            };
            let Some((color_view, color_format, sample_count, resolve_view)) = target_view_info else {
                continue;
            };
            let depth_stencil_info = owner.depth_stencil.and_then(|handle| registry.texture(&handle));
            let depth_format = depth_stencil_info.map(|(_, format)| *format);
            let is_first_target_draw = rendered_targets.insert(owner_path.clone());
            let is_last_target_draw = last_draw_index.get(&owner_path).copied() == Some(index);
            let resolve_target = is_last_target_draw.then_some(resolve_view).flatten();
            let load_op = if is_first_target_draw {
                owner.clear_color
                    .map(|color| wgpu::LoadOp::Clear(wgpu::Color { r: color[0] as f64, g: color[1] as f64, b: color[2] as f64, a: color[3] as f64 }))
                    .unwrap_or(wgpu::LoadOp::Load)
            } else {
                wgpu::LoadOp::Load
            };
            let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                depth_slice: None,
                resolve_target,
                ops: wgpu::Operations { load: load_op, store: wgpu::StoreOp::Store },
            })];
            let depth_stencil_attachment = depth_stencil_info.map(|(view, format)| wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: if is_first_target_draw && owner.clear_color.is_some() { wgpu::LoadOp::Clear(1.0) } else { wgpu::LoadOp::Load },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: format_has_stencil(*format).then_some(wgpu::Operations {
                    load: if is_first_target_draw && owner.clear_color.is_some() { wgpu::LoadOp::Clear(0) } else { wgpu::LoadOp::Load },
                    store: wgpu::StoreOp::Store,
                }),
            });
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderGraphFlatPass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            encode_draw_commands(&mut render_pass, registry, node.commands(), engine.capabilities().max_bind_groups);
            drop(render_pass);
            let _ = (color_format, depth_format, sample_count);
        }
        Ok(())
    }

    /// Thuật toán Biên dịch 2-Phase (1 RenderGraph = 1 RenderPass)
    fn compile_graph(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        engine: &GpuEngine,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        registry: &ResourceRegistry,
        surface_view: Option<&wgpu::TextureView>,
    ) -> Result<(), RenderGraphValidationError> {
        // -------------------------------------------------------------
        // PHASE 1: Đệ quy xử lý tất cả SubGraph con (Bottom-Up)
        // Vẽ các nhánh con ra Offscreen Texture trước khi mở Pass cha
        // -------------------------------------------------------------
        for &node_id in &graph.node_ids {
            let inner_graph = if let Some(RenderNode::SubGraph { graph: inner, .. }) = pool.get(node_id) {
                Some(inner.clone())
            } else {
                None
            };

            if let Some(inner) = inner_graph {
                self.compile_graph(encoder, engine, pool, &inner, registry, surface_view)?;
            }
        }

        // -------------------------------------------------------------
        // PHASE 2: Mở 1 GPU RenderPass DUY NHẤT cho Target của Graph hiện tại
        // -------------------------------------------------------------
        let ordered_ids = graph.ordered_node_ids(pool).map_err(Self::map_graph_flatten_error)?;
        let target_view_info = match &graph.target {
            RenderTarget::Screen => {
                // Surface format thuộc về surface configuration, không được đoán
                // theo backend hoặc theo format mặc định của một cửa sổ cụ thể.
                surface_view
                    .zip(engine.surface_format())
                    .map(|(view, format)| (view, format, 1, None))
                    .or_else(|| registry.texture(&TextureHandle(0)).map(|(v, f)| (v, *f, 1, None)))
            }
            RenderTarget::Offscreen { color, .. } => registry.texture(color).map(|(v, f)| (v, *f, 1, None)),
            RenderTarget::OffscreenMsaa { color, resolve, .. } => registry.texture(color).and_then(|(color_view, format)| {
                registry.texture(resolve).map(|(resolve_view, _)| (color_view, *format, registry.texture_descriptor(color).map_or(1, |d| d.sample_count), Some(resolve_view)))
            }),
        };

        let Some((color_view, color_format, sample_count, resolve_view)) = target_view_info else {
            self.execute_non_render_nodes(encoder, engine, pool, registry, &ordered_ids)?;
            return Ok(());
        };

        let depth_stencil_info = graph.depth_stencil.and_then(|handle| registry.texture(&handle));
        let depth_format = depth_stencil_info.map(|(_, f)| *f);

        let has_draw = ordered_ids.iter().any(|id| pool.get(*id).is_some_and(|node| !node.commands().is_empty()));
        let has_non_render = ordered_ids.iter().any(|id| pool.get(*id).is_some_and(|node| !node.copy_commands().is_empty() || !node.compute_commands().is_empty()));
        if has_draw && has_non_render {
            self.execute_ordered_target_nodes(
                encoder,
                engine,
                pool,
                registry,
                &ordered_ids,
                color_view,
                color_format,
                resolve_view,
                depth_stencil_info.map(|(view, format)| (view, *format)),
                graph.clear_color,
                engine.capabilities().max_bind_groups,
            )?;
            return Ok(());
        }

        // -------------------------------------------------------------
        // 2.1 UPDATE BUNDLES (For nodes that have use_bundle == true)
        // -------------------------------------------------------------
        let node_ids = if graph.reverse_draw_order {
            ordered_ids.iter().rev().copied().collect::<Vec<_>>()
        } else {
            ordered_ids
        };

        for &node_id in &node_ids {
            let expected_bundle_key = pool.get(node_id)
                .map(|node| bundle_cache_key(node, registry, color_format, depth_format, sample_count, self.context_key));
            let Some(node) = pool.get_mut(node_id) else { continue; };
            if node.use_bundle() && (node.is_dirty() || node.bundle().is_none() || node.bundle_key() != expected_bundle_key) {
                let mut bundle_encoder = engine.device().create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                    label: Some("RenderBundleEncoder"),
                    color_formats: &[Some(color_format)],
                    depth_stencil: depth_format.map(|f| wgpu::RenderBundleDepthStencil {
                        format: f,
                        depth_read_only: false,
                        stencil_read_only: false,
                    }),
                    sample_count,
                    multiview: None,
                });

                let mut current_pipeline = None;
                let mut current_bind_groups = vec![None; engine.capabilities().max_bind_groups as usize];

                for cmd in node.commands() {
                    if current_pipeline != Some(cmd.pipeline) {
                        if let Some(pipe) = registry.pipeline(&cmd.pipeline) {
                            bundle_encoder.set_pipeline(pipe);
                            current_pipeline = Some(cmd.pipeline);
                        } else { continue; }
                    }

                    for &(slot, bg_handle, ref offsets) in &cmd.bind_groups {
                        let Some(slot_index) = bind_group_slot_index(slot, engine.capabilities().max_bind_groups) else { continue; };
                        // Rebind if changed, or if there are dynamic offsets (offsets mutate per instance)
                        if current_bind_groups[slot_index] != Some(bg_handle) || !offsets.is_empty() {
                            if let Some(bg) = registry.bind_group(&bg_handle) {
                                bundle_encoder.set_bind_group(slot, bg, offsets);
                                current_bind_groups[slot_index] = Some(bg_handle);
                            }
                        }
                    }

                    match &cmd.action {
                        DrawAction::Indexed { mesh, index_range, instance_range } => {
                            if let Some((vbo, ibo_info, _)) = registry.mesh(mesh) {
                                bundle_encoder.set_vertex_buffer(0, vbo.slice(..));
                                if let Some((ibo, format)) = ibo_info {
                                    bundle_encoder.set_index_buffer(ibo.slice(..), *format);
                                    bundle_encoder.draw_indexed(index_range.clone(), 0, instance_range.clone());
                                } else {
                                    bundle_encoder.draw(index_range.clone(), instance_range.clone());
                                }
                            }
                        }
                        DrawAction::Procedural { vertex_count, instance_range } => {
                            bundle_encoder.draw(0..*vertex_count, instance_range.clone());
                        }
                        DrawAction::Indirect { buffer, offset } => {
                            if let Some(indirect) = registry.buffer(buffer) { bundle_encoder.draw_indirect(indirect, *offset); }
                        }
                        DrawAction::IndexedIndirect { mesh, buffer, offset } => {
                            if let Some((vbo, Some((ibo, format)), _)) = registry.mesh(mesh) {
                                if let Some(indirect) = registry.buffer(buffer) {
                                    bundle_encoder.set_vertex_buffer(0, vbo.slice(..));
                                    bundle_encoder.set_index_buffer(ibo.slice(..), *format);
                                    bundle_encoder.draw_indexed_indirect(indirect, *offset);
                                }
                            }
                        }
                    }
                }

                let bundle = bundle_encoder.finish(&wgpu::RenderBundleDescriptor { label: None });
                match node {
                    RenderNode::DrawBatch { bundle: b, is_dirty, .. } | RenderNode::SubGraph { bundle: b, is_dirty, .. } => {
                        *b = Some(bundle);
                        *is_dirty = false;
                    }
                    RenderNode::ComputeBatch { .. } => unreachable!("compute node cannot create render bundle"),
                    RenderNode::CopyBatch { .. } => unreachable!("copy node cannot create render bundle"),
                    RenderNode::Extension { .. } => unreachable!("extension node cannot create render bundle"),
                }
                node.set_bundle_key(expected_bundle_key.unwrap_or(0));
            }
        }

        // Copy nodes được submit trước compute/render pass của graph hiện tại.
        for &node_id in &node_ids {
            let Some(node) = pool.get(node_id) else { continue; };
            self.dispatch_extension(encoder, engine, registry, pool, node_id)?;
            for command in node.copy_commands() {
                encode_copy_command(encoder, registry, command);
            }
        }

        // Compute nodes được submit trước render pass của graph hiện tại.
        // Khi graph có interleave compute/render semantics, pass model đầy đủ sẽ
        // tách chúng thành execution segments ở compiler tiếp theo.
        for &node_id in &node_ids {
            let Some(node) = pool.get(node_id) else { continue; };
            if node.compute_commands().is_empty() { continue; }
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("RenderGraphComputePass"),
                timestamp_writes: None,
            });
            let mut current_pipeline = None;
            let mut current_bind_groups = vec![None; engine.capabilities().max_bind_groups as usize];
            for command in node.compute_commands() {
                if current_pipeline != Some(command.pipeline) {
                    if let Some(pipeline) = registry.compute_pipeline(&command.pipeline) {
                        compute_pass.set_pipeline(pipeline);
                        current_pipeline = Some(command.pipeline);
                    } else { continue; }
                }
                for &(slot, bind_group, ref offsets) in &command.bind_groups {
                    let Some(slot_index) = bind_group_slot_index(slot, engine.capabilities().max_bind_groups) else { continue; };
                    if current_bind_groups[slot_index] != Some(bind_group) || !offsets.is_empty() {
                        if let Some(group) = registry.bind_group(&bind_group) {
                            compute_pass.set_bind_group(slot, group, offsets);
                            current_bind_groups[slot_index] = Some(bind_group);
                        }
                    }
                }
                if let Some((buffer, offset)) = command.indirect {
                    if let Some(indirect) = registry.buffer(&buffer) { compute_pass.dispatch_workgroups_indirect(indirect, offset); }
                } else {
                    compute_pass.dispatch_workgroups(command.workgroups[0], command.workgroups[1], command.workgroups[2]);
                }
            }
        }

        // -------------------------------------------------------------
        // 2.2 EXECUTE RENDER PASS
        // -------------------------------------------------------------
        let load_op = if let Some(c) = graph.clear_color {
            wgpu::LoadOp::Clear(wgpu::Color { r: c[0] as f64, g: c[1] as f64, b: c[2] as f64, a: c[3] as f64 })
        } else { wgpu::LoadOp::Load };

        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view: color_view,
            depth_slice: None,
            resolve_target: resolve_view,
            ops: wgpu::Operations { load: load_op, store: wgpu::StoreOp::Store },
        })];

        let depth_stencil_attachment = depth_stencil_info.map(|(view, format)| wgpu::RenderPassDepthStencilAttachment {
            view,
            depth_ops: Some(wgpu::Operations {
                load: if graph.clear_color.is_some() { wgpu::LoadOp::Clear(1.0) } else { wgpu::LoadOp::Load },
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: format_has_stencil(*format).then_some(wgpu::Operations {
                load: if graph.clear_color.is_some() { wgpu::LoadOp::Clear(0) } else { wgpu::LoadOp::Load },
                store: wgpu::StoreOp::Store,
            }),
        });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RenderGraphPass"),
            color_attachments: &color_attachments,
            depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let mut current_pipeline = None;
        let mut current_bind_groups = vec![None; engine.capabilities().max_bind_groups as usize];

        for &node_id in &node_ids {
            let Some(node) = pool.get(node_id) else { continue; };
            
            if node.use_bundle() {
                if let Some(bundle) = node.bundle() {
                    render_pass.execute_bundles(std::iter::once(bundle));
                    // State is reset after execute_bundles
                    current_pipeline = None;
                    current_bind_groups.fill(None);
                }
            } else {
                // IMMEDIATE MODE
                for cmd in node.commands() {
                    if current_pipeline != Some(cmd.pipeline) {
                        if let Some(pipe) = registry.pipeline(&cmd.pipeline) {
                            render_pass.set_pipeline(pipe);
                            current_pipeline = Some(cmd.pipeline);
                        } else { continue; }
                    }

                    for &(slot, bg_handle, ref offsets) in &cmd.bind_groups {
                        let Some(slot_index) = bind_group_slot_index(slot, engine.capabilities().max_bind_groups) else { continue; };
                        if current_bind_groups[slot_index] != Some(bg_handle) || !offsets.is_empty() {
                            if let Some(bg) = registry.bind_group(&bg_handle) {
                                render_pass.set_bind_group(slot, bg, offsets);
                                current_bind_groups[slot_index] = Some(bg_handle);
                            }
                        }
                    }

                    match &cmd.action {
                        DrawAction::Indexed { mesh, index_range, instance_range } => {
                            if let Some((vbo, ibo_info, _)) = registry.mesh(mesh) {
                                render_pass.set_vertex_buffer(0, vbo.slice(..));
                                if let Some((ibo, format)) = ibo_info {
                                    render_pass.set_index_buffer(ibo.slice(..), *format);
                                    render_pass.draw_indexed(index_range.clone(), 0, instance_range.clone());
                                } else {
                                    render_pass.draw(index_range.clone(), instance_range.clone());
                                }
                            }
                        }
                        DrawAction::Procedural { vertex_count, instance_range } => {
                            render_pass.draw(0..*vertex_count, instance_range.clone());
                        }
                        DrawAction::Indirect { buffer, offset } => {
                            if let Some(indirect) = registry.buffer(buffer) { render_pass.draw_indirect(indirect, *offset); }
                        }
                        DrawAction::IndexedIndirect { mesh, buffer, offset } => {
                            if let Some((vbo, Some((ibo, format)), _)) = registry.mesh(mesh) {
                                if let Some(indirect) = registry.buffer(buffer) {
                                    render_pass.set_vertex_buffer(0, vbo.slice(..));
                                    render_pass.set_index_buffer(ibo.slice(..), *format);
                                    render_pass.draw_indexed_indirect(indirect, *offset);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn encode_compute_commands(
    encoder: &mut wgpu::CommandEncoder,
    registry: &ResourceRegistry,
    commands: &[crate::graph::ComputeCommand],
    max_bind_groups: u32,
) {
    if commands.is_empty() { return; }
    let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("RenderGraphComputePass"), timestamp_writes: None });
    let mut current_pipeline = None;
    let mut current_bind_groups = vec![None; max_bind_groups as usize];
    for command in commands {
        if current_pipeline != Some(command.pipeline) {
            let Some(pipeline) = registry.compute_pipeline(&command.pipeline) else { continue; };
            compute_pass.set_pipeline(pipeline);
            current_pipeline = Some(command.pipeline);
        }
        for &(slot, bind_group, ref offsets) in &command.bind_groups {
            let Some(slot_index) = bind_group_slot_index(slot, max_bind_groups) else { continue; };
            if current_bind_groups[slot_index] != Some(bind_group) || !offsets.is_empty() {
                let Some(group) = registry.bind_group(&bind_group) else { continue; };
                compute_pass.set_bind_group(slot, group, offsets);
                current_bind_groups[slot_index] = Some(bind_group);
            }
        }
        if let Some((buffer, offset)) = command.indirect {
            if let Some(indirect) = registry.buffer(&buffer) { compute_pass.dispatch_workgroups_indirect(indirect, offset); }
        } else {
            compute_pass.dispatch_workgroups(command.workgroups[0], command.workgroups[1], command.workgroups[2]);
        }
    }
}

fn encode_draw_commands(
    render_pass: &mut wgpu::RenderPass<'_>,
    registry: &ResourceRegistry,
    commands: &[crate::graph::DrawCommand],
    max_bind_groups: u32,
) {
    let mut current_pipeline = None;
    let mut current_bind_groups = vec![None; max_bind_groups as usize];
    for command in commands {
        if current_pipeline != Some(command.pipeline) {
            let Some(pipeline) = registry.pipeline(&command.pipeline) else { continue; };
            render_pass.set_pipeline(pipeline);
            current_pipeline = Some(command.pipeline);
        }
        for &(slot, bind_group, ref offsets) in &command.bind_groups {
            let Some(slot_index) = bind_group_slot_index(slot, max_bind_groups) else { continue; };
            if current_bind_groups[slot_index] != Some(bind_group) || !offsets.is_empty() {
                let Some(group) = registry.bind_group(&bind_group) else { continue; };
                render_pass.set_bind_group(slot, group, offsets);
                current_bind_groups[slot_index] = Some(bind_group);
            }
        }
        match &command.action {
            DrawAction::Indexed { mesh, index_range, instance_range } => {
                let Some((vbo, ibo_info, _)) = registry.mesh(mesh) else { continue; };
                render_pass.set_vertex_buffer(0, vbo.slice(..));
                if let Some((ibo, format)) = ibo_info {
                    render_pass.set_index_buffer(ibo.slice(..), *format);
                    render_pass.draw_indexed(index_range.clone(), 0, instance_range.clone());
                } else {
                    render_pass.draw(index_range.clone(), instance_range.clone());
                }
            }
            DrawAction::Procedural { vertex_count, instance_range } => render_pass.draw(0..*vertex_count, instance_range.clone()),
            DrawAction::Indirect { buffer, offset } => {
                if let Some(indirect) = registry.buffer(buffer) { render_pass.draw_indirect(indirect, *offset); }
            }
            DrawAction::IndexedIndirect { mesh, buffer, offset } => {
                if let Some((vbo, Some((ibo, format)), _)) = registry.mesh(mesh) {
                    if let Some(indirect) = registry.buffer(buffer) {
                        render_pass.set_vertex_buffer(0, vbo.slice(..));
                        render_pass.set_index_buffer(ibo.slice(..), *format);
                        render_pass.draw_indexed_indirect(indirect, *offset);
                    }
                }
            }
        }
    }
}

fn validate_graph(
    registry: &ResourceRegistry,
    pool: &RenderNodePool,
    graph: &RenderGraph,
    max_bind_groups: u32,
    extension_dispatchers: &ExtensionDispatchRegistry,
) -> Result<(), RenderGraphValidationError> {
    graph.flatten(pool).map_err(|error| match error {
        crate::graph::GraphFlattenError::MissingNode(node) => RenderGraphValidationError::MissingNode(node),
        crate::graph::GraphFlattenError::Cycle(node) => RenderGraphValidationError::DependencyCycle(node),
        crate::graph::GraphFlattenError::DependencyNodeOutsideGraph(node) => RenderGraphValidationError::DependencyOutsideGraph(node),
    })?;
    match graph.target {
        RenderTarget::Screen => {}
        RenderTarget::Offscreen { color, width, height } => {
            if width == 0 || height == 0 {
                return Err(RenderGraphValidationError::InvalidTargetSize { width, height });
            }
            if !registry.contains_texture(&color) {
                return Err(RenderGraphValidationError::MissingTexture(color));
            }
            if let Some(descriptor) = registry.texture_descriptor(&color) {
                if descriptor.width != width || descriptor.height != height {
                    return Err(RenderGraphValidationError::TargetSizeMismatch {
                        handle: color,
                        width,
                        height,
                        actual_width: descriptor.width,
                        actual_height: descriptor.height,
                    });
                }
                let required = wgpu::TextureUsages::RENDER_ATTACHMENT;
                if !descriptor.usage.contains(required) {
                    return Err(RenderGraphValidationError::MissingTextureUsage {
                        handle: color,
                        required_usage: required.bits(),
                        actual_usage: descriptor.usage.bits(),
                    });
                }
                if descriptor.sample_count != 1 {
                    return Err(RenderGraphValidationError::UnsupportedSampleCount { handle: color, actual: descriptor.sample_count });
                }
            }
        }
        RenderTarget::OffscreenMsaa { color, resolve, width, height } => {
            if width == 0 || height == 0 {
                return Err(RenderGraphValidationError::InvalidTargetSize { width, height });
            }
            let color_descriptor = registry.texture_descriptor(&color).ok_or(RenderGraphValidationError::MissingTexture(color))?;
            if !registry.contains_texture(&color) {
                return Err(RenderGraphValidationError::MissingTexture(color));
            }
            if color_descriptor.width != width || color_descriptor.height != height {
                return Err(RenderGraphValidationError::TargetSizeMismatch {
                    handle: color, width, height,
                    actual_width: color_descriptor.width, actual_height: color_descriptor.height,
                });
            }
            if color_descriptor.sample_count <= 1 {
                return Err(RenderGraphValidationError::UnsupportedSampleCount { handle: color, actual: color_descriptor.sample_count });
            }
            if !color_descriptor.usage.contains(wgpu::TextureUsages::RENDER_ATTACHMENT) {
                return Err(RenderGraphValidationError::MissingTextureUsage {
                    handle: color,
                    required_usage: wgpu::TextureUsages::RENDER_ATTACHMENT.bits(),
                    actual_usage: color_descriptor.usage.bits(),
                });
            }
            let resolve_descriptor = registry.texture_descriptor(&resolve).ok_or(RenderGraphValidationError::MissingResolveTexture(resolve))?;
            if resolve_descriptor.width != width || resolve_descriptor.height != height {
                return Err(RenderGraphValidationError::ResolveSizeMismatch {
                    color_width: width, color_height: height,
                    resolve_width: resolve_descriptor.width, resolve_height: resolve_descriptor.height,
                });
            }
            if resolve_descriptor.sample_count != 1 {
                return Err(RenderGraphValidationError::InvalidResolveSampleCount { handle: resolve, actual: resolve_descriptor.sample_count });
            }
            if resolve_descriptor.format != color_descriptor.format {
                return Err(RenderGraphValidationError::ResolveFormatMismatch { color: color_descriptor.format, resolve: resolve_descriptor.format });
            }
            if !resolve_descriptor.usage.contains(wgpu::TextureUsages::RENDER_ATTACHMENT) {
                return Err(RenderGraphValidationError::MissingTextureUsage {
                    handle: resolve,
                    required_usage: wgpu::TextureUsages::RENDER_ATTACHMENT.bits(),
                    actual_usage: resolve_descriptor.usage.bits(),
                });
            }
        }
    }

    let target_sample_count = match graph.target {
        RenderTarget::OffscreenMsaa { color, .. } => registry.texture_descriptor(&color).map_or(1, |descriptor| descriptor.sample_count),
        _ => 1,
    };
    if let Some(depth) = graph.depth_stencil {
        if !registry.contains_texture(&depth) {
            return Err(RenderGraphValidationError::MissingTexture(depth));
        }
        if let Some(descriptor) = registry.texture_descriptor(&depth) {
            let required = wgpu::TextureUsages::RENDER_ATTACHMENT;
            if !descriptor.usage.contains(required) {
                return Err(RenderGraphValidationError::MissingTextureUsage {
                    handle: depth,
                    required_usage: required.bits(),
                    actual_usage: descriptor.usage.bits(),
                });
            }
            if descriptor.sample_count != target_sample_count {
                return Err(RenderGraphValidationError::DepthSampleCountMismatch {
                    handle: depth, expected: target_sample_count, actual: descriptor.sample_count,
                });
            }
        }
    }

    for &node_id in &graph.node_ids {
        let node = pool.get(node_id).ok_or(RenderGraphValidationError::MissingNode(node_id))?;
        if let RenderNode::Extension { extension, usages } = node {
            let Some(dispatcher) = extension_dispatchers.get(extension) else {
                return Err(RenderGraphValidationError::UnsupportedExtension(extension.clone()));
            };
            dispatcher.validate(usages).map_err(|error| RenderGraphValidationError::ExtensionValidation {
                extension: extension.clone(),
                error,
            })?;
        }
        for usage in graph.resource_usages(&node_id) {
            match usage.resource {
                GraphResource::Buffer(handle) if !registry.contains_buffer(&handle) => {
                    return Err(RenderGraphValidationError::MissingUsageBuffer(handle));
                }
                GraphResource::Texture(handle) if !registry.contains_texture(&handle) => {
                    return Err(RenderGraphValidationError::MissingUsageTexture(handle));
                }
                _ => {}
            }
        }
        for command in node.commands() {
            if !registry.contains_pipeline(&command.pipeline) {
                return Err(RenderGraphValidationError::MissingPipeline(command.pipeline));
            }
            for &(slot, bind_group, ref offsets) in &command.bind_groups {
                if bind_group_slot_index(slot, max_bind_groups).is_none() {
                    return Err(RenderGraphValidationError::InvalidBindGroupSlot { slot, max_slots: max_bind_groups });
                }
                if !registry.contains_bind_group(&bind_group) {
                    return Err(RenderGraphValidationError::MissingBindGroup(bind_group));
                }
                validate_bind_group_offsets(registry, bind_group, offsets)?;
                validate_render_pipeline_layout(registry, command.pipeline, slot, bind_group)?;
            }
            if let DrawAction::Indexed { mesh, .. } = command.action {
                if !registry.contains_mesh(&mesh) {
                    return Err(RenderGraphValidationError::MissingMesh(mesh));
                }
            }
            match command.action {
                DrawAction::Indirect { buffer, offset } => validate_indirect_buffer(registry, buffer, offset, 16)?,
                DrawAction::IndexedIndirect { mesh, buffer, offset } => {
                    let Some((_, Some(_), _)) = registry.mesh(&mesh) else {
                        if !registry.contains_mesh(&mesh) { return Err(RenderGraphValidationError::MissingMesh(mesh)); }
                        return Err(RenderGraphValidationError::MissingIndexBuffer(mesh));
                    };
                    validate_indirect_buffer(registry, buffer, offset, 20)?;
                }
                _ => {}
            }
        }
        for command in node.compute_commands() {
            if !registry.contains_compute_pipeline(&command.pipeline) {
                return Err(RenderGraphValidationError::MissingComputePipeline(command.pipeline));
            }
            for &(slot, bind_group, ref offsets) in &command.bind_groups {
                if bind_group_slot_index(slot, max_bind_groups).is_none() {
                    return Err(RenderGraphValidationError::InvalidBindGroupSlot { slot, max_slots: max_bind_groups });
                }
                if !registry.contains_bind_group(&bind_group) {
                    return Err(RenderGraphValidationError::MissingBindGroup(bind_group));
                }
                validate_bind_group_offsets(registry, bind_group, offsets)?;
                validate_compute_pipeline_layout(registry, command.pipeline, slot, bind_group)?;
            }
            if let Some((buffer, offset)) = command.indirect {
                validate_indirect_buffer(registry, buffer, offset, 12)?;
            }
        }
        for command in node.copy_commands() {
            match command {
                CopyCommand::BufferToBuffer { source, destination, source_offset, destination_offset, size } => {
                    let Some(source_buffer) = registry.buffer(source) else {
                        return Err(RenderGraphValidationError::MissingBuffer(*source));
                    };
                    let Some(destination_buffer) = registry.buffer(destination) else {
                        return Err(RenderGraphValidationError::MissingBuffer(*destination));
                    };
                    if let Some(descriptor) = registry.buffer_descriptor(source) {
                        let required = wgpu::BufferUsages::COPY_SRC;
                        if !descriptor.usage.contains(required) {
                            return Err(RenderGraphValidationError::MissingBufferUsage { handle: *source, required_usage: required.bits(), actual_usage: descriptor.usage.bits() });
                        }
                    }
                    if let Some(descriptor) = registry.buffer_descriptor(destination) {
                        let required = wgpu::BufferUsages::COPY_DST;
                        if !descriptor.usage.contains(required) {
                            return Err(RenderGraphValidationError::MissingBufferUsage { handle: *destination, required_usage: required.bits(), actual_usage: descriptor.usage.bits() });
                        }
                    }
                    validate_copy_range(*source, *source_offset, *size, source_buffer.size())?;
                    validate_copy_range(*destination, *destination_offset, *size, destination_buffer.size())?;
                }
                CopyCommand::TextureToTexture { source, destination, source_mip_level, destination_mip_level, source_origin, destination_origin, extent } => {
                    validate_texture_copy(registry, *source, *destination, *source_mip_level, *destination_mip_level, *source_origin, *destination_origin, *extent, crate::graph::TextureAspect::All)?;
                }
                CopyCommand::TextureToTextureAspect { source, destination, source_mip_level, destination_mip_level, source_origin, destination_origin, extent, aspect } => {
                    validate_texture_copy(registry, *source, *destination, *source_mip_level, *destination_mip_level, *source_origin, *destination_origin, *extent, *aspect)?;
                }
            }
        }
        if let RenderNode::SubGraph { graph: child, .. } = node {
            validate_graph(registry, pool, child, max_bind_groups, extension_dispatchers)?;
        }
    }
    Ok(())
}

fn encode_copy_command(encoder: &mut wgpu::CommandEncoder, registry: &ResourceRegistry, command: &CopyCommand) {
    match command {
        CopyCommand::BufferToBuffer { source, destination, source_offset, destination_offset, size } => {
            let Some(source_buffer) = registry.buffer(source) else { return; };
            let Some(destination_buffer) = registry.buffer(destination) else { return; };
            encoder.copy_buffer_to_buffer(source_buffer, *source_offset, destination_buffer, *destination_offset, *size);
        }
        CopyCommand::TextureToTexture { source, destination, source_mip_level, destination_mip_level, source_origin, destination_origin, extent } => {
            encode_texture_copy(encoder, registry, *source, *destination, *source_mip_level, *destination_mip_level, *source_origin, *destination_origin, *extent, crate::graph::TextureAspect::All);
        }
        CopyCommand::TextureToTextureAspect { source, destination, source_mip_level, destination_mip_level, source_origin, destination_origin, extent, aspect } => {
            encode_texture_copy(encoder, registry, *source, *destination, *source_mip_level, *destination_mip_level, *source_origin, *destination_origin, *extent, *aspect);
        }
    }
}

fn encode_texture_copy(
    encoder: &mut wgpu::CommandEncoder,
    registry: &ResourceRegistry,
    source: TextureHandle,
    destination: TextureHandle,
    source_mip_level: u32,
    destination_mip_level: u32,
    source_origin: [u32; 3],
    destination_origin: [u32; 3],
    extent: [u32; 3],
    aspect: crate::graph::TextureAspect,
) {
            let Some(source_texture) = registry.owned_texture(&source) else { return; };
            let Some(destination_texture) = registry.owned_texture(&destination) else { return; };
            let origin = |value: [u32; 3]| wgpu::Origin3d { x: value[0], y: value[1], z: value[2] };
            let extent = wgpu::Extent3d { width: extent[0], height: extent[1], depth_or_array_layers: extent[2] };
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo { texture: source_texture, mip_level: source_mip_level, origin: origin(source_origin), aspect: to_wgpu_texture_aspect(aspect) },
                wgpu::TexelCopyTextureInfo { texture: destination_texture, mip_level: destination_mip_level, origin: origin(destination_origin), aspect: to_wgpu_texture_aspect(aspect) },
                extent,
            );
}

fn to_wgpu_texture_aspect(aspect: crate::graph::TextureAspect) -> wgpu::TextureAspect {
    match aspect {
        crate::graph::TextureAspect::All => wgpu::TextureAspect::All,
        crate::graph::TextureAspect::DepthOnly => wgpu::TextureAspect::DepthOnly,
        crate::graph::TextureAspect::StencilOnly => wgpu::TextureAspect::StencilOnly,
    }
}

fn validate_texture_copy(
    registry: &ResourceRegistry,
    source: TextureHandle,
    destination: TextureHandle,
    source_mip_level: u32,
    destination_mip_level: u32,
    source_origin: [u32; 3],
    destination_origin: [u32; 3],
    extent: [u32; 3],
    aspect: crate::graph::TextureAspect,
) -> Result<(), RenderGraphValidationError> {
    if !registry.contains_texture(&source) { return Err(RenderGraphValidationError::MissingTexture(source)); }
    if !registry.contains_texture(&destination) { return Err(RenderGraphValidationError::MissingTexture(destination)); }
    let Some(source_texture) = registry.owned_texture(&source) else { return Err(RenderGraphValidationError::MissingOwnedTexture(source)); };
    let Some(destination_texture) = registry.owned_texture(&destination) else { return Err(RenderGraphValidationError::MissingOwnedTexture(destination)); };
    let _ = (source_texture, destination_texture);
    let Some(source_descriptor) = registry.texture_descriptor(&source) else { return Err(RenderGraphValidationError::MissingTextureDescriptor(source)); };
    let Some(destination_descriptor) = registry.texture_descriptor(&destination) else { return Err(RenderGraphValidationError::MissingTextureDescriptor(destination)); };
    if source_descriptor.format != destination_descriptor.format {
        return Err(RenderGraphValidationError::TextureCopyFormatMismatch { source_handle: source, destination_handle: destination });
    }
    if !texture_supports_aspect(source_descriptor.format, aspect) {
        return Err(RenderGraphValidationError::InvalidTextureAspect { handle: source, aspect });
    }
    if !texture_supports_aspect(destination_descriptor.format, aspect) {
        return Err(RenderGraphValidationError::InvalidTextureAspect { handle: destination, aspect });
    }
    let copy_src = wgpu::TextureUsages::COPY_SRC;
    let copy_dst = wgpu::TextureUsages::COPY_DST;
    if !source_descriptor.usage.contains(copy_src) {
        return Err(RenderGraphValidationError::MissingTextureUsage { handle: source, required_usage: copy_src.bits(), actual_usage: source_descriptor.usage.bits() });
    }
    if !destination_descriptor.usage.contains(copy_dst) {
        return Err(RenderGraphValidationError::MissingTextureUsage { handle: destination, required_usage: copy_dst.bits(), actual_usage: destination_descriptor.usage.bits() });
    }
    if extent.iter().any(|value| *value == 0) { return Err(RenderGraphValidationError::InvalidTextureCopyExtent { extent }); }
    validate_texture_mip(source, source_mip_level, source_origin, extent, source_descriptor)?;
    validate_texture_mip(destination, destination_mip_level, destination_origin, extent, destination_descriptor)?;
    Ok(())
}

fn texture_supports_aspect(format: wgpu::TextureFormat, aspect: crate::graph::TextureAspect) -> bool {
    match aspect {
        crate::graph::TextureAspect::All => true,
        crate::graph::TextureAspect::DepthOnly => matches!(
            format,
            wgpu::TextureFormat::Depth16Unorm
                | wgpu::TextureFormat::Depth24Plus
                | wgpu::TextureFormat::Depth24PlusStencil8
                | wgpu::TextureFormat::Depth32Float
                | wgpu::TextureFormat::Depth32FloatStencil8
        ),
        crate::graph::TextureAspect::StencilOnly => matches!(
            format,
            wgpu::TextureFormat::Stencil8
                | wgpu::TextureFormat::Depth24PlusStencil8
                | wgpu::TextureFormat::Depth32FloatStencil8
        ),
    }
}

fn validate_texture_mip(
    handle: TextureHandle,
    mip_level: u32,
    origin: [u32; 3],
    extent: [u32; 3],
    descriptor: &crate::resources::registry::TextureResourceDescriptor,
) -> Result<(), RenderGraphValidationError> {
    if mip_level >= descriptor.mip_level_count {
        return Err(RenderGraphValidationError::InvalidTextureMipLevel { handle, mip_level, mip_count: descriptor.mip_level_count });
    }
    let mip_extent = [
        (descriptor.width >> mip_level).max(1),
        (descriptor.height >> mip_level).max(1),
        descriptor.depth_or_array_layers,
    ];
    let in_bounds = origin.iter().zip(extent).zip(mip_extent).all(|((origin, extent), dimension)| origin.checked_add(extent).is_some_and(|end| end <= dimension));
    if !in_bounds {
        return Err(RenderGraphValidationError::InvalidTextureCopyRange { handle, origin, extent, mip_extent });
    }
    Ok(())
}

fn validate_copy_range(
    handle: BufferHandle,
    offset: u64,
    size: u64,
    buffer_size: u64,
) -> Result<(), RenderGraphValidationError> {
    let end = offset.checked_add(size).ok_or(RenderGraphValidationError::InvalidCopyRange {
        handle, offset, size, buffer_size,
    })?;
    if end > buffer_size {
        return Err(RenderGraphValidationError::InvalidCopyRange { handle, offset, size, buffer_size });
    }
    Ok(())
}

fn validate_indirect_buffer(
    registry: &ResourceRegistry,
    handle: BufferHandle,
    offset: u64,
    size: u64,
) -> Result<(), RenderGraphValidationError> {
    let Some(buffer) = registry.buffer(&handle) else { return Err(RenderGraphValidationError::MissingIndirectBuffer(handle)); };
    if offset % 4 != 0 || offset.checked_add(size).is_none_or(|end| end > buffer.size()) {
        return Err(RenderGraphValidationError::InvalidIndirectRange { handle, offset, size });
    }
    if let Some(descriptor) = registry.buffer_descriptor(&handle) {
        let required = wgpu::BufferUsages::INDIRECT;
        if !descriptor.usage.contains(required) {
            return Err(RenderGraphValidationError::MissingIndirectBufferUsage { handle, required_usage: required.bits(), actual_usage: descriptor.usage.bits() });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    use super::{bind_group_slot_index, bundle_cache_key, format_has_stencil, texture_supports_aspect, validate_copy_range, validate_indirect_buffer, RenderGraphExecutor, RenderGraphProfilingError, RenderGraphValidationError};
    use crate::api::GpuEngineBuilder;
    use crate::memory::SubmissionTracker;
    use crate::graph::{ComputeCommand, CopyCommand, DrawAction, DrawCommand, GraphResource, RenderGraph, RenderNode, RenderNodePool, RenderTarget, ResourceAccess, ResourceSubresource};
    use crate::resources::{BindGroupHandle, BufferHandle, BufferResourceDescriptor, ComputePipelineHandle, PipelineHandle, RenderNodeId, ResourceRegistry, TextureHandle, TextureResourceDescriptor};

    struct CountingDispatcher {
        descriptor: crate::extensions::ExtensionDescriptor,
        calls: Arc<AtomicUsize>,
    }

    impl crate::extensions::ExtensionDispatcher for CountingDispatcher {
        fn descriptor(&self) -> crate::extensions::ExtensionDescriptor { self.descriptor.clone() }

        fn encode(
            &self,
            context: crate::extensions::ExtensionExecutionContext<'_, '_>,
        ) -> Result<(), crate::extensions::ExtensionExecutionError> {
            assert_eq!(context.usages().len(), 0);
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    fn invalid_bind_group_slot_does_not_index_state_cache() {
        assert_eq!(bind_group_slot_index(0, 4), Some(0));
        assert_eq!(bind_group_slot_index(3, 4), Some(3));
        assert_eq!(bind_group_slot_index(4, 4), None);
        assert_eq!(bind_group_slot_index(7, 8), Some(7));
        assert_eq!(bind_group_slot_index(u32::MAX, 8), None);
    }

    #[test]
    fn extension_without_dispatch_fails_closed_before_resource_lookup() {
        let extension_id = crate::extensions::ExtensionId::new("test.unhandled").unwrap();
        let mut pool = RenderNodePool::new();
        let node = pool.alloc_extension(extension_id.clone(), Vec::new());
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(node);

        assert_eq!(
            RenderGraphExecutor::new().validate(&ResourceRegistry::new(), &pool, &graph),
            Err(RenderGraphValidationError::UnsupportedExtension(extension_id))
        );
    }

    #[test]
    fn registered_extension_dispatches_once_in_no_target_path() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let extension_id = crate::extensions::ExtensionId::new("test.counting").unwrap();
        let mut dispatchers = crate::extensions::ExtensionDispatchRegistry::new();
        dispatchers.register(Arc::new(CountingDispatcher {
            descriptor: crate::extensions::ExtensionDescriptor { id: extension_id.clone(), version: 1 },
            calls: calls.clone(),
        })).unwrap();

        let mut pool = RenderNodePool::new();
        let node = pool.alloc_extension(extension_id, Vec::new());
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(node);

        RenderGraphExecutor::with_extension_dispatchers(dispatchers)
            .execute_checked(&engine, &ResourceRegistry::new(), &mut pool, &graph)
            .unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn profiled_execution_is_opt_in_and_has_typed_backend_fallback() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let Ok(mut profiler) = crate::api::TimestampQueryPool::new(engine.device(), 2) else {
            return;
        };
        let resolve_buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("profiling-resolve-test"),
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let result = RenderGraphExecutor::new().execute_checked_with_timestamp(
            &engine,
            &ResourceRegistry::new(),
            &mut RenderNodePool::new(),
            &RenderGraph::new(RenderTarget::Screen),
            &mut profiler,
            &resolve_buffer,
            0,
        );
        match result {
            Ok(profiled) => assert_eq!(profiled.report.flattened_nodes, 0),
            Err(RenderGraphProfilingError::Profiling(crate::api::ProfilingError::UnsupportedEncoderTimestamps)) => {}
            Err(error) => panic!("unexpected profiled execution error: {error:?}"),
        }
    }

    #[test]
    fn tracked_profiled_execution_reserves_pool_until_host_completion() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let Ok(mut profiler) = crate::api::TimestampQueryPool::new(engine.device(), 2) else {
            return;
        };
        let resolve_buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("tracked-profiling-resolve-test"),
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let mut tracker = SubmissionTracker::new();
        let result = RenderGraphExecutor::new().execute_checked_with_timestamp_tracked(
            &engine,
            &ResourceRegistry::new(),
            &mut RenderNodePool::new(),
            &RenderGraph::new(RenderTarget::Screen),
            &mut profiler,
            &resolve_buffer,
            0,
            &mut tracker,
        );
        match result {
            Ok(profiled) => {
                let submission = profiled.tracking_submission.expect("tracked API must reserve a submission");
                assert_eq!(submission, crate::memory::SubmissionId(1));
                assert_eq!(profiler.allocate_span(), Err(crate::api::ProfilingError::InFlight));
                assert!(!profiler.reset_after(&tracker).unwrap());
                tracker.mark_completed(submission);
                assert!(profiler.reset_after(&tracker).unwrap());
            }
            Err(RenderGraphProfilingError::Profiling(crate::api::ProfilingError::UnsupportedEncoderTimestamps)) => {}
            Err(error) => panic!("unexpected tracked profiling error: {error:?}"),
        }
    }

    #[test]
    fn execution_report_counts_flattened_commands_and_usages() {
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        let draw = graph.add_batch(&mut pool, vec![DrawCommand::new(
            PipelineHandle(1),
            DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 },
        )]);
        let compute = graph.add_compute_batch(&mut pool, vec![ComputeCommand::new(
            ComputePipelineHandle(2), [1, 1, 1],
        )]);
        graph.add_copy_batch(&mut pool, vec![CopyCommand::buffer_to_buffer(
            BufferHandle(3), BufferHandle(4), 16,
        )]);
        graph.declare_resource_usage(draw, GraphResource::Buffer(BufferHandle(5)), ResourceAccess::Read);
        graph.declare_resource_usage(compute, GraphResource::Buffer(BufferHandle(6)), ResourceAccess::Write);

        let counts = RenderGraphExecutor::execution_counts_for_graph(&pool, &graph).unwrap();
        assert_eq!(counts, (3, 1, 1, 1, 0, 2));
    }

    #[test]
    fn stencil_aspect_detection_is_format_specific() {
        assert!(format_has_stencil(wgpu::TextureFormat::Stencil8));
        assert!(format_has_stencil(wgpu::TextureFormat::Depth24PlusStencil8));
        assert!(format_has_stencil(wgpu::TextureFormat::Depth32FloatStencil8));
        assert!(!format_has_stencil(wgpu::TextureFormat::Depth24Plus));
        assert!(!format_has_stencil(wgpu::TextureFormat::Depth32Float));
    }

    #[test]
    fn texture_copy_aspect_support_is_format_specific() {
        use crate::graph::TextureAspect;
        assert!(texture_supports_aspect(wgpu::TextureFormat::Depth24PlusStencil8, TextureAspect::DepthOnly));
        assert!(texture_supports_aspect(wgpu::TextureFormat::Depth24PlusStencil8, TextureAspect::StencilOnly));
        assert!(texture_supports_aspect(wgpu::TextureFormat::Stencil8, TextureAspect::StencilOnly));
        assert!(!texture_supports_aspect(wgpu::TextureFormat::Rgba8Unorm, TextureAspect::DepthOnly));
        assert!(!texture_supports_aspect(wgpu::TextureFormat::Depth32Float, TextureAspect::StencilOnly));
    }

    #[test]
    fn indirect_buffer_validation_checks_alignment_range_and_usage() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let mut registry = ResourceRegistry::new();
        let buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("indirect_validation"), size: 64,
            usage: wgpu::BufferUsages::INDIRECT, mapped_at_creation: false,
        });
        registry.insert_buffer_with_descriptor(BufferHandle(70), buffer, BufferResourceDescriptor { size: 64, usage: wgpu::BufferUsages::INDIRECT }).unwrap();
        assert!(validate_indirect_buffer(&registry, BufferHandle(70), 0, 16).is_ok());
        assert!(matches!(validate_indirect_buffer(&registry, BufferHandle(70), 2, 16), Err(RenderGraphValidationError::InvalidIndirectRange { .. })));
        assert!(matches!(validate_indirect_buffer(&registry, BufferHandle(70), 52, 16), Err(RenderGraphValidationError::InvalidIndirectRange { .. })));
    }

    #[test]
    fn validation_rejects_missing_offscreen_target() {
        let graph = RenderGraph::new(RenderTarget::Offscreen {
            color: TextureHandle(9),
            width: 64,
            height: 64,
        });
        let result = RenderGraphExecutor::new().validate(
            &ResourceRegistry::new(),
            &RenderNodePool::new(),
            &graph,
        );

        assert_eq!(result, Err(RenderGraphValidationError::MissingTexture(TextureHandle(9))));
    }

    #[test]
    fn public_execute_rejects_invalid_graph_before_submit() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let graph = RenderGraph::new(RenderTarget::Offscreen {
            color: TextureHandle(9),
            width: 64,
            height: 64,
        });
        let result = RenderGraphExecutor::new().execute(
            &engine,
            &ResourceRegistry::new(),
            &mut RenderNodePool::new(),
            &graph,
        );

        assert_eq!(result.err(), Some(RenderGraphValidationError::MissingTexture(TextureHandle(9))));
    }

    #[test]
    fn validate_with_device_exposes_adapter_aware_contract() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let graph = RenderGraph::new(RenderTarget::Offscreen {
            color: TextureHandle(9),
            width: 64,
            height: 64,
        });
        let result = RenderGraphExecutor::new().validate_with_device(
            &engine,
            &ResourceRegistry::new(),
            &RenderNodePool::new(),
            &graph,
        );
        assert_eq!(result, Err(RenderGraphValidationError::MissingTexture(TextureHandle(9))));
    }

    #[test]
    fn validation_rejects_zero_sized_target_before_resource_lookup() {
        let graph = RenderGraph::new(RenderTarget::Offscreen {
            color: TextureHandle(9),
            width: 0,
            height: 64,
        });
        let result = RenderGraphExecutor::new().validate(
            &ResourceRegistry::new(),
            &RenderNodePool::new(),
            &graph,
        );

        assert_eq!(
            result,
            Err(RenderGraphValidationError::InvalidTargetSize { width: 0, height: 64 })
        );
    }

    #[test]
    fn bundle_key_changes_when_pipeline_version_changes() {
        let node = RenderNode::new_batch(vec![DrawCommand::new(
            PipelineHandle(7),
            DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 },
        )]);
        let mut registry = ResourceRegistry::new();
        let first = bundle_cache_key(&node, &registry, wgpu::TextureFormat::Rgba8Unorm, None, 1, 0);
        registry.mark_pipeline_changed(PipelineHandle(7));
        let second = bundle_cache_key(&node, &registry, wgpu::TextureFormat::Rgba8Unorm, None, 1, 0);

        assert_ne!(first, second);
        let single_sample = bundle_cache_key(&node, &registry, wgpu::TextureFormat::Rgba8Unorm, None, 1, 0);
        let msaa = bundle_cache_key(&node, &registry, wgpu::TextureFormat::Rgba8Unorm, None, 4, 0);
        assert_ne!(single_sample, msaa);
        let context_a = bundle_cache_key(&node, &registry, wgpu::TextureFormat::Rgba8Unorm, None, 1, 11);
        let context_b = bundle_cache_key(&node, &registry, wgpu::TextureFormat::Rgba8Unorm, None, 1, 22);
        assert_ne!(context_a, context_b);
    }

    #[test]
    fn validation_rejects_graph_target_dimension_mismatch() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("descriptor_test"),
            size: wgpu::Extent3d { width: 128, height: 64, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let mut registry = ResourceRegistry::new();
        registry.insert_texture_with_descriptor(
            TextureHandle(1),
            texture.create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 128,
                height: 64,
                depth_or_array_layers: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1,
                sample_count: 1,
            },
            1024,
        ).unwrap();
        let graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(1), width: 64, height: 64 });

        assert_eq!(
            RenderGraphExecutor::new().validate(&registry, &RenderNodePool::new(), &graph),
            Err(RenderGraphValidationError::TargetSizeMismatch {
                handle: TextureHandle(1), width: 64, height: 64, actual_width: 128, actual_height: 64,
            })
        );

        let texture_without_attachment = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("usage_test"),
            size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        registry.insert_texture_with_descriptor(
            TextureHandle(2),
            texture_without_attachment.create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 64, height: 64, depth_or_array_layers: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                mip_level_count: 1, sample_count: 1,
            },
            1024,
        ).unwrap();
        let usage_graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(2), width: 64, height: 64 });
        assert!(matches!(
            RenderGraphExecutor::new().validate(&registry, &RenderNodePool::new(), &usage_graph),
            Err(RenderGraphValidationError::MissingTextureUsage { handle: TextureHandle(2), .. })
        ));

        let multisampled = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("multisampled_target"),
            size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        registry.insert_texture_with_descriptor(
            TextureHandle(3),
            multisampled.create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 64, height: 64, depth_or_array_layers: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1, sample_count: 4,
            },
            1024,
        ).unwrap();
        let multisample_graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(3), width: 64, height: 64 });
        assert_eq!(
            RenderGraphExecutor::new().validate(&registry, &RenderNodePool::new(), &multisample_graph),
            Err(RenderGraphValidationError::UnsupportedSampleCount { handle: TextureHandle(3), actual: 4 })
        );
    }

    #[test]
    fn validation_accepts_msaa_attachment_with_single_sample_resolve() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let make_texture = |label, sample_count| engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            view_formats: &[],
        });
        let mut registry = ResourceRegistry::new();
        registry.insert_texture_with_descriptor(
            TextureHandle(10),
            make_texture("msaa_color", 4).create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor { width: 64, height: 64, depth_or_array_layers: 1, format: wgpu::TextureFormat::Rgba8Unorm, usage, mip_level_count: 1, sample_count: 4 },
            1024,
        ).unwrap();
        registry.insert_texture_with_descriptor(
            TextureHandle(11),
            make_texture("resolve_color", 1).create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor { width: 64, height: 64, depth_or_array_layers: 1, format: wgpu::TextureFormat::Rgba8Unorm, usage, mip_level_count: 1, sample_count: 1 },
            1024,
        ).unwrap();

        let graph = RenderGraph::new(RenderTarget::OffscreenMsaa {
            color: TextureHandle(10), resolve: TextureHandle(11), width: 64, height: 64,
        });
        assert!(RenderGraphExecutor::new().validate(&registry, &RenderNodePool::new(), &graph).is_ok());
    }

    #[test]
    fn execute_msaa_target_with_resolve_attachment() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let make_texture = |label, sample_count| engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label), size: wgpu::Extent3d { width: 8, height: 8, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, usage, view_formats: &[],
        });
        let color = make_texture("msaa_execute_color", 4);
        let resolve = make_texture("msaa_execute_resolve", 1);
        let mut registry = ResourceRegistry::new();
        registry.insert_texture_with_descriptor(
            TextureHandle(20), color.create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor { width: 8, height: 8, depth_or_array_layers: 1, format: wgpu::TextureFormat::Rgba8Unorm, usage, mip_level_count: 1, sample_count: 4 }, 1024,
        ).unwrap();
        registry.insert_texture_with_descriptor(
            TextureHandle(21), resolve.create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor { width: 8, height: 8, depth_or_array_layers: 1, format: wgpu::TextureFormat::Rgba8Unorm, usage, mip_level_count: 1, sample_count: 1 }, 1024,
        ).unwrap();
        let graph = RenderGraph::new(RenderTarget::OffscreenMsaa { color: TextureHandle(20), resolve: TextureHandle(21), width: 8, height: 8 });
        let submission = RenderGraphExecutor::new().execute(&engine, &registry, &mut RenderNodePool::new(), &graph).unwrap();
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None });
    }

    #[test]
    fn execute_msaa_target_with_matching_depth_attachment() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let color_usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let color = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("msaa_depth_color"), size: wgpu::Extent3d { width: 8, height: 8, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 4, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, usage: color_usage, view_formats: &[],
        });
        let resolve = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("msaa_depth_resolve"), size: wgpu::Extent3d { width: 8, height: 8, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, usage: color_usage, view_formats: &[],
        });
        let depth = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("msaa_depth_attachment"), size: wgpu::Extent3d { width: 8, height: 8, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 4, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8, usage: color_usage, view_formats: &[],
        });
        let mut registry = ResourceRegistry::new();
        let descriptor = |format, sample_count| TextureResourceDescriptor {
            width: 8, height: 8, depth_or_array_layers: 1, format, usage: color_usage,
            mip_level_count: 1, sample_count,
        };
        registry.insert_texture_with_descriptor(TextureHandle(30), color.create_view(&wgpu::TextureViewDescriptor::default()), descriptor(wgpu::TextureFormat::Rgba8Unorm, 4), 1024).unwrap();
        registry.insert_texture_with_descriptor(TextureHandle(31), resolve.create_view(&wgpu::TextureViewDescriptor::default()), descriptor(wgpu::TextureFormat::Rgba8Unorm, 1), 1024).unwrap();
        registry.insert_texture_with_descriptor(TextureHandle(32), depth.create_view(&wgpu::TextureViewDescriptor::default()), descriptor(wgpu::TextureFormat::Depth24PlusStencil8, 4), 1024).unwrap();
        let mut graph = RenderGraph::new(RenderTarget::OffscreenMsaa { color: TextureHandle(30), resolve: TextureHandle(31), width: 8, height: 8 });
        graph.depth_stencil = Some(TextureHandle(32));
        let submission = RenderGraphExecutor::new().execute(&engine, &registry, &mut RenderNodePool::new(), &graph).unwrap();
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None });
    }

    #[test]
    fn validation_rejects_compute_node_without_pipeline() {
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_compute_batch(
            &mut pool,
            vec![ComputeCommand::new(ComputePipelineHandle(42), [1, 1, 1])],
        );

        assert_eq!(
            RenderGraphExecutor::new().validate(&ResourceRegistry::new(), &pool, &graph),
            Err(RenderGraphValidationError::MissingComputePipeline(ComputePipelineHandle(42)))
        );
    }

    #[test]
    fn validation_rejects_copy_node_without_buffer() {
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_copy_batch(
            &mut pool,
            vec![CopyCommand::buffer_to_buffer(BufferHandle(1), BufferHandle(2), 16)],
        );

        assert_eq!(
            RenderGraphExecutor::new().validate(&ResourceRegistry::new(), &pool, &graph),
            Err(RenderGraphValidationError::MissingBuffer(BufferHandle(1)))
        );
    }

    #[test]
    fn validation_rejects_declared_usage_without_resource() {
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        let node = graph.add_copy_batch(&mut pool, vec![]);
        graph.declare_resource_usage(node, GraphResource::Buffer(BufferHandle(404)), ResourceAccess::Read);
        assert_eq!(RenderGraphExecutor::new().validate(&ResourceRegistry::new(), &pool, &graph), Err(RenderGraphValidationError::MissingUsageBuffer(BufferHandle(404))));
    }

    #[test]
    fn declared_resource_usage_is_preserved_on_graph() {
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        let node = RenderNodeId(9);
        graph.declare_resource_usage(node, GraphResource::Texture(TextureHandle(7)), ResourceAccess::ReadWrite);
        assert_eq!(graph.resource_usages(&node), &[crate::graph::ResourceUsage { resource: GraphResource::Texture(TextureHandle(7)), access: ResourceAccess::ReadWrite, subresource: ResourceSubresource::Whole }]);
    }

    #[test]
    fn validation_rejects_buffer_copy_with_missing_usage_bits() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let source = engine.device().create_buffer(&wgpu::BufferDescriptor { label: Some("invalid_copy_source"), size: 16, usage: wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let destination = engine.device().create_buffer(&wgpu::BufferDescriptor { label: Some("invalid_copy_destination"), size: 16, usage: wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
        let mut registry = ResourceRegistry::new();
        registry.insert_buffer_with_descriptor(BufferHandle(1), source, BufferResourceDescriptor { size: 16, usage: wgpu::BufferUsages::COPY_DST }).unwrap();
        registry.insert_buffer_with_descriptor(BufferHandle(2), destination, BufferResourceDescriptor { size: 16, usage: wgpu::BufferUsages::COPY_SRC }).unwrap();
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_copy_batch(&mut pool, vec![CopyCommand::buffer_to_buffer(BufferHandle(1), BufferHandle(2), 4)]);
        assert_eq!(RenderGraphExecutor::new().validate(&registry, &pool, &graph), Err(RenderGraphValidationError::MissingBufferUsage { handle: BufferHandle(1), required_usage: wgpu::BufferUsages::COPY_SRC.bits(), actual_usage: wgpu::BufferUsages::COPY_DST.bits() }));
    }

    #[test]
    fn copy_range_validation_checks_overflow_and_bounds() {
        assert!(validate_copy_range(BufferHandle(1), 8, 8, 16).is_ok());
        assert!(matches!(
            validate_copy_range(BufferHandle(1), 12, 8, 16),
            Err(RenderGraphValidationError::InvalidCopyRange { .. })
        ));
        assert!(matches!(
            validate_copy_range(BufferHandle(1), u64::MAX, 1, u64::MAX),
            Err(RenderGraphValidationError::InvalidCopyRange { .. })
        ));
    }

    #[test]
    fn copy_only_graph_executes_without_render_target() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let source = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("copy_source"), size: 4,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let destination = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("copy_destination"), size: 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        engine.queue().write_buffer(&source, 0, &[7, 8, 9, 10]);

        let mut registry = ResourceRegistry::new();
        registry.insert_buffer(BufferHandle(1), source);
        registry.insert_buffer(BufferHandle(2), destination);
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_copy_batch(&mut pool, vec![CopyCommand::buffer_to_buffer(BufferHandle(1), BufferHandle(2), 4)]);

        let submission = RenderGraphExecutor::new().execute_checked(&engine, &registry, &mut pool, &graph).unwrap();
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission.clone()), timeout: None });
        let destination = registry.buffer(&BufferHandle(2)).unwrap();
        let slice = destination.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| { let _ = sender.send(result); });
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None });
        receiver.recv().unwrap().unwrap();
        assert_eq!(&*slice.get_mapped_range().unwrap(), &[7, 8, 9, 10]);
    }

    #[test]
    fn texture_copy_graph_executes_and_preserves_pixels() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let usage = wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST;
        let descriptor = TextureResourceDescriptor {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            mip_level_count: 1,
            sample_count: 1,
        };
        let create_texture = |label| engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            view_formats: &[],
        });
        let source = create_texture("texture_copy_source");
        let destination = create_texture("texture_copy_destination");
        let pixels = [
            255, 0, 0, 255, 0, 255, 0, 255,
            0, 0, 255, 255, 255, 255, 255, 255,
        ];
        engine.queue().write_texture(
            source.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(8), rows_per_image: Some(2) },
            wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 1 },
        );

        let mut registry = ResourceRegistry::new();
        registry.insert_owned_texture(TextureHandle(1), source, descriptor, 1024).unwrap();
        registry.insert_owned_texture(TextureHandle(2), destination, descriptor, 1024).unwrap();
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_copy_batch(&mut pool, vec![CopyCommand::texture_to_texture(TextureHandle(1), TextureHandle(2), [2, 2, 1])]);

        let submission = RenderGraphExecutor::new().execute_checked(&engine, &registry, &mut pool, &graph).unwrap();
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None });
        let (actual, width, height) = engine.read_texture_to_bytes_with_format(registry.owned_texture(&TextureHandle(2)).unwrap(), wgpu::TextureFormat::Rgba8Unorm).unwrap();
        assert_eq!((width, height), (2, 2));
        assert_eq!(actual, pixels);
    }

    #[test]
    fn texture_copy_validation_rejects_missing_ownership_and_out_of_bounds_extent() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let usage = wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST;
        let descriptor = TextureResourceDescriptor {
            width: 4, height: 4, depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8Unorm, usage,
            mip_level_count: 1, sample_count: 1,
        };
        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("validation_texture"),
            size: wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, usage, view_formats: &[],
        });
        let mut registry = ResourceRegistry::new();
        registry.insert_owned_texture(TextureHandle(1), texture, descriptor, 1024).unwrap();
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_copy_batch(&mut pool, vec![CopyCommand::texture_to_texture(TextureHandle(1), TextureHandle(2), [1, 1, 1])]);
        assert_eq!(RenderGraphExecutor::new().validate(&registry, &pool, &graph), Err(RenderGraphValidationError::MissingTexture(TextureHandle(2))));

        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("validation_texture_destination"),
            size: wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, usage, view_formats: &[],
        });
        registry.insert_owned_texture(TextureHandle(2), texture, descriptor, 1024).unwrap();
        graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_copy_batch(&mut pool, vec![CopyCommand::texture_to_texture(TextureHandle(1), TextureHandle(2), [5, 3, 1])]);
        assert!(matches!(RenderGraphExecutor::new().validate(&registry, &pool, &graph), Err(RenderGraphValidationError::InvalidTextureCopyRange { .. })));
    }

    #[test]
    fn target_graph_with_interleaved_copy_and_draw_uses_ordered_segments() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ordered_segments_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                "@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> { var p = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0)); return vec4<f32>(p[i], 0.0, 1.0); } @fragment fn fs() -> @location(0) vec4<f32> { return vec4<f32>(1.0, 0.0, 0.0, 1.0); }",
            )),
        });
        let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ordered_segments_pipeline"),
            layout: None,
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), buffers: &[], compilation_options: Default::default() },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
            multiview_mask: None,
            cache: None,
        });
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST;
        let descriptor = TextureResourceDescriptor { width: 2, height: 2, depth_or_array_layers: 1, format: wgpu::TextureFormat::Rgba8Unorm, usage, mip_level_count: 1, sample_count: 1 };
        let make_texture = |label| engine.device().create_texture(&wgpu::TextureDescriptor { label: Some(label), size: wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage, view_formats: &[] });
        let mut registry = ResourceRegistry::new();
        registry.insert_owned_texture(TextureHandle(1), make_texture("ordered_source"), descriptor, 1024).unwrap();
        registry.insert_owned_texture(TextureHandle(2), make_texture("ordered_copy_destination"), descriptor, 1024).unwrap();
        registry.insert_owned_texture(TextureHandle(3), make_texture("ordered_target"), descriptor, 1024).unwrap();
        registry.insert_pipeline(PipelineHandle(1), pipeline);
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(3), width: 2, height: 2 });
        graph.add_copy_batch(&mut pool, vec![CopyCommand::texture_to_texture(TextureHandle(1), TextureHandle(2), [2, 2, 1])]);
        graph.add_batch(&mut pool, vec![DrawCommand::new(PipelineHandle(1), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 })]);
        let submission = RenderGraphExecutor::new().execute_checked(&engine, &registry, &mut pool, &graph).unwrap();
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None });
        let (pixels, _, _) = engine.read_texture_to_bytes_with_format(registry.owned_texture(&TextureHandle(3)).unwrap(), wgpu::TextureFormat::Rgba8Unorm).unwrap();
        assert_eq!(&pixels[0..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn compute_only_graph_executes_storage_update_without_render_target() {
        let engine = pollster::block_on(GpuEngineBuilder::new().with_required_limits(wgpu::Limits::default()).build()).unwrap();
        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_test"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                "@group(0) @binding(0) var<storage, read_write> data: array<u32>; @compute @workgroup_size(1) fn main() { data[0] = data[0] + 1u; }",
            )),
        });
        let layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_test_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("compute_test_pipeline_layout"), bind_group_layouts: &[Some(&layout)], immediate_size: 0,
        });
        let pipeline = engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_test_pipeline"), layout: Some(&pipeline_layout), module: &shader,
            entry_point: Some("main"), compilation_options: Default::default(), cache: None,
        });
        let buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("compute_test_buffer"), size: 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let staging = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("compute_test_staging"), size: 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        engine.queue().write_buffer(&buffer, 0, bytemuck::bytes_of(&0u32));
        let bind_group = engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute_test_bind_group"), layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
        });
        let mut registry = ResourceRegistry::new();
        registry.insert_buffer(BufferHandle(1), buffer);
        registry.insert_buffer(BufferHandle(2), staging);
        registry.insert_compute_pipeline(ComputePipelineHandle(1), pipeline);
        registry.insert_bind_group(BindGroupHandle(1), bind_group);
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_compute_batch(&mut pool, vec![ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(0, BindGroupHandle(1), vec![])]);
        graph.add_copy_batch(&mut pool, vec![CopyCommand::buffer_to_buffer(BufferHandle(1), BufferHandle(2), 4)]);

        let submission = RenderGraphExecutor::new().execute_checked(&engine, &registry, &mut pool, &graph).unwrap();
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission.clone()), timeout: None });
        let staging = registry.buffer(&BufferHandle(2)).unwrap();
        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| { let _ = sender.send(result); });
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None });
        receiver.recv().unwrap().unwrap();
        let bytes = slice.get_mapped_range().unwrap();
        assert_eq!(u32::from_ne_bytes(bytes[0..4].try_into().unwrap()), 1);
    }

    #[test]
    fn flattened_execution_preserves_root_before_nested_compute_order() {
        let engine = pollster::block_on(GpuEngineBuilder::new().with_required_limits(wgpu::Limits::default()).build()).unwrap();
        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nested_order_compute"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                "@group(0) @binding(0) var<storage, read_write> data: array<u32>; @compute @workgroup_size(1) fn main() { data[0] = data[0] + 1u; }",
            )),
        });
        let layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nested_order_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nested_order_pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("nested_order_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let make_buffer = |label, usage| engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: 4,
            usage,
            mapped_at_creation: false,
        });
        let source = make_buffer("nested_order_source", wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST);
        let shared = make_buffer("nested_order_shared", wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST);
        let staging = make_buffer("nested_order_staging", wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ);
        engine.queue().write_buffer(&source, 0, bytemuck::bytes_of(&7u32));
        engine.queue().write_buffer(&shared, 0, bytemuck::bytes_of(&0u32));
        let bind_group = engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nested_order_bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: shared.as_entire_binding() }],
        });
        let mut registry = ResourceRegistry::new();
        registry.insert_buffer(BufferHandle(1), source);
        registry.insert_buffer(BufferHandle(2), shared);
        registry.insert_buffer(BufferHandle(3), staging);
        registry.insert_compute_pipeline(ComputePipelineHandle(1), pipeline);
        registry.insert_bind_group(BindGroupHandle(1), bind_group);

        let mut pool = RenderNodePool::new();
        let mut child = RenderGraph::new(RenderTarget::Screen);
        let child_compute = child.add_compute_batch(&mut pool, vec![
            ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(0, BindGroupHandle(1), vec![]),
        ]);
        child.declare_resource_usage(child_compute, GraphResource::Buffer(BufferHandle(2)), ResourceAccess::ReadWrite);
        let mut root = RenderGraph::new(RenderTarget::Screen);
        root.add_copy_batch(&mut pool, vec![CopyCommand::buffer_to_buffer(BufferHandle(1), BufferHandle(2), 4)]);
        root.add_subgraph(&mut pool, "nested-order", child, vec![]);
        root.add_copy_batch(&mut pool, vec![CopyCommand::buffer_to_buffer(BufferHandle(2), BufferHandle(3), 4)]);

        let submission = RenderGraphExecutor::new().execute_checked(&engine, &registry, &mut pool, &root).unwrap();
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None });
        let staging = registry.buffer(&BufferHandle(3)).unwrap();
        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| { let _ = sender.send(result); });
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
        receiver.recv().unwrap().unwrap();
        let bytes = slice.get_mapped_range().unwrap();
        assert_eq!(u32::from_ne_bytes(bytes[0..4].try_into().unwrap()), 8);
    }

    #[test]
    fn validation_checks_descriptor_aware_dynamic_offsets() {
        let engine = pollster::block_on(GpuEngineBuilder::new().with_required_limits(wgpu::Limits::default()).build()).unwrap();
        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("dynamic_offset_validation_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                "@group(0) @binding(0) var<uniform> value: u32; @compute @workgroup_size(1) fn main() { _ = value; }",
            )),
        });
        let layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dynamic_offset_validation_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("dynamic_offset_validation_pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("dynamic_offset_validation_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let alignment = engine.capabilities().min_uniform_buffer_offset_alignment;
        let buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("dynamic_offset_validation_buffer"),
            size: alignment as u64 * 2,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        let bind_group = engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dynamic_offset_validation_bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(4),
                }),
            }],
        });
        let mismatched_bind_group = engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dynamic_offset_mismatched_bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(4),
                }),
            }],
        });
        let mut registry = ResourceRegistry::new();
        registry.insert_compute_pipeline_with_layout_descriptor(
            ComputePipelineHandle(1),
            pipeline,
            crate::resources::PipelineLayoutResourceDescriptor {
                bind_group_layout_signatures: vec![Some(7)],
            },
        );
        registry
            .insert_bind_group_with_descriptor(
                BindGroupHandle(1),
                bind_group,
                crate::resources::BindGroupResourceDescriptor {
                    dynamic_offset_count: 1,
                    dynamic_offset_alignment: alignment,
                    layout_signature: 7,
                },
            )
            .unwrap();
        registry
            .insert_bind_group_with_descriptor(
                BindGroupHandle(2),
                mismatched_bind_group,
                crate::resources::BindGroupResourceDescriptor {
                    dynamic_offset_count: 1,
                    dynamic_offset_alignment: alignment,
                    layout_signature: 8,
                },
            )
            .unwrap();
        let mut pool = RenderNodePool::new();
        let mut valid = RenderGraph::new(RenderTarget::Screen);
        valid.add_compute_batch(&mut pool, vec![ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(0, BindGroupHandle(1), vec![alignment])]);
        assert_eq!(RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &valid), Ok(()));
        let mut invalid = RenderGraph::new(RenderTarget::Screen);
        invalid.add_compute_batch(&mut pool, vec![ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(0, BindGroupHandle(1), vec![1])]);
        assert_eq!(
            RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &invalid),
            Err(RenderGraphValidationError::InvalidDynamicOffsetAlignment {
                handle: BindGroupHandle(1), offset: 1, alignment,
            })
        );
        let mut mismatched = RenderGraph::new(RenderTarget::Screen);
        mismatched.add_compute_batch(&mut pool, vec![ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(0, BindGroupHandle(2), vec![alignment])]);
        assert_eq!(
            RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &mismatched),
            Err(RenderGraphValidationError::ComputePipelineLayoutMismatch {
                pipeline: ComputePipelineHandle(1), slot: 0, expected: Some(7), actual: Some(8),
            })
        );
    }
}
