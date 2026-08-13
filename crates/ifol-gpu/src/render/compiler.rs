use thiserror::Error;
use crate::api::GpuEngine;
use crate::render::graph::{DrawAction, RenderGraph, RenderNode, RenderNodePool, RenderTarget};
use crate::render::handle::{BindGroupHandle, MeshHandle, PipelineHandle, RenderNodeId, TextureHandle};
use crate::render::registry::ResourceRegistry;

pub struct RenderGraphExecutor;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RenderGraphValidationError {
    #[error("render node {0:?} does not exist in the node pool")]
    MissingNode(RenderNodeId),
    #[error("texture resource {0:?} is missing")]
    MissingTexture(TextureHandle),
    #[error("pipeline resource {0:?} is missing")]
    MissingPipeline(PipelineHandle),
    #[error("mesh resource {0:?} is missing")]
    MissingMesh(MeshHandle),
    #[error("bind group resource {0:?} is missing")]
    MissingBindGroup(BindGroupHandle),
    #[error("bind group slot {0} is outside the supported range 0..4")]
    InvalidBindGroupSlot(u32),
    #[error("render target dimensions must be non-zero, got {width}x{height}")]
    InvalidTargetSize { width: u32, height: u32 },
}

fn bind_group_slot_index(slot: u32) -> Option<usize> {
    (slot < 4).then_some(slot as usize)
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
            return;
        };

        let depth_stencil_info = graph.depth_stencil.and_then(|handle| registry.textures.get(&handle));
        let depth_format = depth_stencil_info.map(|(_, f)| *f);

        // -------------------------------------------------------------
        // 2.1 UPDATE BUNDLES (For nodes that have use_bundle == true)
        // -------------------------------------------------------------
        let node_ids = if graph.reverse_draw_order {
            graph.node_ids.iter().rev().copied().collect::<Vec<_>>()
        } else {
            graph.node_ids.clone()
        };

        for &node_id in &node_ids {
            let Some(node) = pool.get_mut(node_id) else { continue; };
            if node.use_bundle() && (node.is_dirty() || node.bundle().is_none()) {
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
    match graph.target {
        RenderTarget::Screen => {}
        RenderTarget::Offscreen { color, width, height } => {
            if width == 0 || height == 0 {
                return Err(RenderGraphValidationError::InvalidTargetSize { width, height });
            }
            if !registry.textures.contains_key(&color) {
                return Err(RenderGraphValidationError::MissingTexture(color));
            }
        }
    }

    if let Some(depth) = graph.depth_stencil {
        if !registry.textures.contains_key(&depth) {
            return Err(RenderGraphValidationError::MissingTexture(depth));
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
        if let RenderNode::SubGraph { graph: child, .. } = node {
            validate_graph(registry, pool, child)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{bind_group_slot_index, RenderGraphExecutor, RenderGraphValidationError};
    use crate::render::{RenderGraph, RenderNodePool, RenderTarget, ResourceRegistry, TextureHandle};

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
}
