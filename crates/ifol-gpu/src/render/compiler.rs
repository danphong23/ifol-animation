use thiserror::Error;
use std::hash::{Hash, Hasher};
use crate::api::GpuEngine;
use crate::render::graph::{DrawAction, RenderGraph, RenderNode, RenderNodePool, RenderTarget};
use crate::render::handle::{BindGroupHandle, BufferHandle, ComputePipelineHandle, MeshHandle, PipelineHandle, RenderNodeId, TextureHandle};
use crate::render::registry::ResourceRegistry;

pub struct RenderGraphExecutor;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RenderGraphValidationError {
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
    #[error("copy range for buffer {handle:?} exceeds buffer size: offset {offset}, size {size}, buffer size {buffer_size}")]
    InvalidCopyRange { handle: BufferHandle, offset: u64, size: u64, buffer_size: u64 },
    #[error("mesh resource {0:?} is missing")]
    MissingMesh(MeshHandle),
    #[error("bind group resource {0:?} is missing")]
    MissingBindGroup(BindGroupHandle),
    #[error("bind group slot {0} is outside the supported range 0..4")]
    InvalidBindGroupSlot(u32),
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
}

fn bind_group_slot_index(slot: u32) -> Option<usize> {
    (slot < 4).then_some(slot as usize)
}

fn bundle_cache_key(
    node: &RenderNode,
    registry: &ResourceRegistry,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    color_format.hash(&mut hasher);
    depth_format.hash(&mut hasher);
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
        Self
    }

    /// Kiểm tra graph trước khi tạo command buffer. Đây là API được khuyến nghị
    /// cho host muốn nhận lỗi typed thay vì behavior silent-skip của prototype.
    pub fn validate(
        &self,
        registry: &ResourceRegistry,
        pool: &RenderNodePool,
        graph: &RenderGraph,
    ) -> Result<(), RenderGraphValidationError> {
        validate_graph(registry, pool, graph)
    }

    pub fn execute_checked(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
    ) -> Result<wgpu::SubmissionIndex, RenderGraphValidationError> {
        self.validate(registry, pool, graph)?;
        Ok(self.execute(engine, registry, pool, graph))
    }

    /// Biên dịch RenderGraph thành các lệnh gọi WGPU và đẩy xuống GPU Queue.
    pub fn execute(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
    ) -> wgpu::SubmissionIndex {
        self.execute_with_surface(engine, registry, pool, graph, None)
    }

    /// Biên dịch RenderGraph với Surface Texture View chỉ định (khi vẽ trực tiếp ra cửa sổ)
    pub fn execute_with_surface(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        surface_view: Option<&wgpu::TextureView>,
    ) -> wgpu::SubmissionIndex {
        let mut encoder = engine.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("RenderGraphEncoder"),
        });

        // Duyệt 2-Phase cây RenderGraph
        self.compile_graph(&mut encoder, engine, pool, graph, registry, surface_view);

        // Submit toàn bộ khối lệnh (Command Buffer) lên GPU 1 lần duy nhất
        engine.queue().submit(std::iter::once(encoder.finish()))
    }

    fn execute_non_render_nodes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        engine: &GpuEngine,
        pool: &RenderNodePool,
        registry: &ResourceRegistry,
        node_ids: &[RenderNodeId],
    ) {
        for &node_id in node_ids {
            let Some(node) = pool.get(node_id) else { continue; };
            for command in node.copy_commands() {
                let Some(source) = registry.buffers.get(&command.source) else { continue; };
                let Some(destination) = registry.buffers.get(&command.destination) else { continue; };
                encoder.copy_buffer_to_buffer(source, command.source_offset, destination, command.destination_offset, command.size);
            }
            if !node.compute_commands().is_empty() {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("RenderGraphComputePass"), timestamp_writes: None,
                });
                let mut current_pipeline = None;
                let mut current_bind_groups = [None; 4];
                for command in node.compute_commands() {
                    if current_pipeline != Some(command.pipeline) {
                        if let Some(pipeline) = registry.compute_pipelines.get(&command.pipeline) {
                            compute_pass.set_pipeline(pipeline);
                            current_pipeline = Some(command.pipeline);
                        } else { continue; }
                    }
                    for &(slot, bind_group, ref offsets) in &command.bind_groups {
                        let Some(slot_index) = bind_group_slot_index(slot) else { continue; };
                        if current_bind_groups[slot_index] != Some(bind_group) || !offsets.is_empty() {
                            if let Some(group) = registry.bind_groups.get(&bind_group) {
                                compute_pass.set_bind_group(slot, group, offsets);
                                current_bind_groups[slot_index] = Some(bind_group);
                            }
                        }
                    }
                    compute_pass.dispatch_workgroups(command.workgroups[0], command.workgroups[1], command.workgroups[2]);
                }
            }
        }
        let _ = engine;
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
    ) {
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
                self.compile_graph(encoder, engine, pool, &inner, registry, surface_view);
            }
        }

        // -------------------------------------------------------------
        // PHASE 2: Mở 1 GPU RenderPass DUY NHẤT cho Target của Graph hiện tại
        // -------------------------------------------------------------
        let ordered_ids = graph.ordered_node_ids(pool).unwrap_or_else(|_| graph.node_ids.clone());
        let target_view_info = match &graph.target {
            RenderTarget::Screen => {
                // Surface format thuộc về surface configuration, không được đoán
                // theo backend hoặc theo format mặc định của một cửa sổ cụ thể.
                surface_view
                    .zip(engine.surface_format())
                    .or_else(|| registry.textures.get(&TextureHandle(0)).map(|(v, f)| (v, *f)))
            }
            RenderTarget::Offscreen { color, .. } => registry.textures.get(color).map(|(v, f)| (v, *f)),
        };

        let Some((color_view, color_format)) = target_view_info else {
            self.execute_non_render_nodes(encoder, engine, pool, registry, &ordered_ids);
            return;
        };

        let depth_stencil_info = graph.depth_stencil.and_then(|handle| registry.textures.get(&handle));
        let depth_format = depth_stencil_info.map(|(_, f)| *f);

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
                .map(|node| bundle_cache_key(node, registry, color_format, depth_format));
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
                    sample_count: 1,
                    multiview: None,
                });

                let mut current_pipeline = None;
                let mut current_bind_groups = [None; 4];

                for cmd in node.commands() {
                    if current_pipeline != Some(cmd.pipeline) {
                        if let Some(pipe) = registry.pipelines.get(&cmd.pipeline) {
                            bundle_encoder.set_pipeline(pipe);
                            current_pipeline = Some(cmd.pipeline);
                        } else { continue; }
                    }

                    for &(slot, bg_handle, ref offsets) in &cmd.bind_groups {
                        let Some(slot_index) = bind_group_slot_index(slot) else { continue; };
                        // Rebind if changed, or if there are dynamic offsets (offsets mutate per instance)
                        if current_bind_groups[slot_index] != Some(bg_handle) || !offsets.is_empty() {
                            if let Some(bg) = registry.bind_groups.get(&bg_handle) {
                                bundle_encoder.set_bind_group(slot, bg, offsets);
                                current_bind_groups[slot_index] = Some(bg_handle);
                            }
                        }
                    }

                    match &cmd.action {
                        DrawAction::Indexed { mesh, index_range, instance_range } => {
                            if let Some((vbo, ibo_info, _)) = registry.meshes.get(mesh) {
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
                }
                node.set_bundle_key(expected_bundle_key.unwrap_or(0));
            }
        }

        // Copy nodes được submit trước compute/render pass của graph hiện tại.
        for &node_id in &node_ids {
            let Some(node) = pool.get(node_id) else { continue; };
            for command in node.copy_commands() {
                let Some(source) = registry.buffers.get(&command.source) else { continue; };
                let Some(destination) = registry.buffers.get(&command.destination) else { continue; };
                encoder.copy_buffer_to_buffer(
                    source,
                    command.source_offset,
                    destination,
                    command.destination_offset,
                    command.size,
                );
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
            let mut current_bind_groups = [None; 4];
            for command in node.compute_commands() {
                if current_pipeline != Some(command.pipeline) {
                    if let Some(pipeline) = registry.compute_pipelines.get(&command.pipeline) {
                        compute_pass.set_pipeline(pipeline);
                        current_pipeline = Some(command.pipeline);
                    } else { continue; }
                }
                for &(slot, bind_group, ref offsets) in &command.bind_groups {
                    let Some(slot_index) = bind_group_slot_index(slot) else { continue; };
                    if current_bind_groups[slot_index] != Some(bind_group) || !offsets.is_empty() {
                        if let Some(group) = registry.bind_groups.get(&bind_group) {
                            compute_pass.set_bind_group(slot, group, offsets);
                            current_bind_groups[slot_index] = Some(bind_group);
                        }
                    }
                }
                compute_pass.dispatch_workgroups(command.workgroups[0], command.workgroups[1], command.workgroups[2]);
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
            resolve_target: None,
            ops: wgpu::Operations { load: load_op, store: wgpu::StoreOp::Store },
        })];

        let depth_stencil_attachment = depth_stencil_info.map(|(view, _)| wgpu::RenderPassDepthStencilAttachment {
            view,
            depth_ops: Some(wgpu::Operations {
                load: if graph.clear_color.is_some() { wgpu::LoadOp::Clear(1.0) } else { wgpu::LoadOp::Load },
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
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
        let mut current_bind_groups = [None; 4];

        for &node_id in &node_ids {
            let Some(node) = pool.get(node_id) else { continue; };
            
            if node.use_bundle() {
                if let Some(bundle) = node.bundle() {
                    render_pass.execute_bundles(std::iter::once(bundle));
                    // State is reset after execute_bundles
                    current_pipeline = None;
                    current_bind_groups = [None; 4];
                }
            } else {
                // IMMEDIATE MODE
                for cmd in node.commands() {
                    if current_pipeline != Some(cmd.pipeline) {
                        if let Some(pipe) = registry.pipelines.get(&cmd.pipeline) {
                            render_pass.set_pipeline(pipe);
                            current_pipeline = Some(cmd.pipeline);
                        } else { continue; }
                    }

                    for &(slot, bg_handle, ref offsets) in &cmd.bind_groups {
                        let Some(slot_index) = bind_group_slot_index(slot) else { continue; };
                        if current_bind_groups[slot_index] != Some(bg_handle) || !offsets.is_empty() {
                            if let Some(bg) = registry.bind_groups.get(&bg_handle) {
                                render_pass.set_bind_group(slot, bg, offsets);
                                current_bind_groups[slot_index] = Some(bg_handle);
                            }
                        }
                    }

                    match &cmd.action {
                        DrawAction::Indexed { mesh, index_range, instance_range } => {
                            if let Some((vbo, ibo_info, _)) = registry.meshes.get(mesh) {
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
) -> Result<(), RenderGraphValidationError> {
    graph.flatten(pool).map_err(|error| match error {
        crate::render::graph::GraphFlattenError::MissingNode(node) => RenderGraphValidationError::MissingNode(node),
        crate::render::graph::GraphFlattenError::Cycle(node) => RenderGraphValidationError::DependencyCycle(node),
        crate::render::graph::GraphFlattenError::DependencyNodeOutsideGraph(node) => RenderGraphValidationError::DependencyOutsideGraph(node),
    })?;
    match graph.target {
        RenderTarget::Screen => {}
        RenderTarget::Offscreen { color, width, height } => {
            if width == 0 || height == 0 {
                return Err(RenderGraphValidationError::InvalidTargetSize { width, height });
            }
            if !registry.textures.contains_key(&color) {
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
            }
        }
    }

    if let Some(depth) = graph.depth_stencil {
        if !registry.textures.contains_key(&depth) {
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
        }
    }

    for &node_id in &graph.node_ids {
        let node = pool.get(node_id).ok_or(RenderGraphValidationError::MissingNode(node_id))?;
        for command in node.commands() {
            if !registry.pipelines.contains_key(&command.pipeline) {
                return Err(RenderGraphValidationError::MissingPipeline(command.pipeline));
            }
            for &(slot, bind_group, _) in &command.bind_groups {
                if bind_group_slot_index(slot).is_none() {
                    return Err(RenderGraphValidationError::InvalidBindGroupSlot(slot));
                }
                if !registry.bind_groups.contains_key(&bind_group) {
                    return Err(RenderGraphValidationError::MissingBindGroup(bind_group));
                }
            }
            if let DrawAction::Indexed { mesh, .. } = command.action {
                if !registry.meshes.contains_key(&mesh) {
                    return Err(RenderGraphValidationError::MissingMesh(mesh));
                }
            }
        }
        for command in node.compute_commands() {
            if !registry.compute_pipelines.contains_key(&command.pipeline) {
                return Err(RenderGraphValidationError::MissingComputePipeline(command.pipeline));
            }
            for &(slot, bind_group, _) in &command.bind_groups {
                if bind_group_slot_index(slot).is_none() {
                    return Err(RenderGraphValidationError::InvalidBindGroupSlot(slot));
                }
                if !registry.bind_groups.contains_key(&bind_group) {
                    return Err(RenderGraphValidationError::MissingBindGroup(bind_group));
                }
            }
        }
        for command in node.copy_commands() {
            let Some(source) = registry.buffers.get(&command.source) else {
                return Err(RenderGraphValidationError::MissingBuffer(command.source));
            };
            let Some(destination) = registry.buffers.get(&command.destination) else {
                return Err(RenderGraphValidationError::MissingBuffer(command.destination));
            };
            validate_copy_range(command.source, command.source_offset, command.size, source.size())?;
            validate_copy_range(command.destination, command.destination_offset, command.size, destination.size())?;
        }
        if let RenderNode::SubGraph { graph: child, .. } = node {
            validate_graph(registry, pool, child)?;
        }
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

#[cfg(test)]
mod tests {
    use super::{bind_group_slot_index, bundle_cache_key, validate_copy_range, RenderGraphExecutor, RenderGraphValidationError};
    use crate::api::GpuEngineBuilder;
    use crate::render::{BindGroupHandle, BufferHandle, ComputeCommand, CopyCommand, DrawAction, DrawCommand, ComputePipelineHandle, PipelineHandle, RenderGraph, RenderNode, RenderNodePool, RenderTarget, ResourceRegistry, TextureHandle, TextureResourceDescriptor};

    #[test]
    fn invalid_bind_group_slot_does_not_index_state_cache() {
        assert_eq!(bind_group_slot_index(0), Some(0));
        assert_eq!(bind_group_slot_index(3), Some(3));
        assert_eq!(bind_group_slot_index(4), None);
        assert_eq!(bind_group_slot_index(u32::MAX), None);
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
        let first = bundle_cache_key(&node, &registry, wgpu::TextureFormat::Rgba8Unorm, None);
        registry.mark_pipeline_changed(PipelineHandle(7));
        let second = bundle_cache_key(&node, &registry, wgpu::TextureFormat::Rgba8Unorm, None);

        assert_ne!(first, second);
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
        registry.bind_groups.insert(BindGroupHandle(1), bind_group);
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
}
