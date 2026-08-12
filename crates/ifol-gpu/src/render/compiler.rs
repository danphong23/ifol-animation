use crate::api::GpuEngine;
use crate::render::graph::{DrawAction, DrawCommand, RenderGraph, RenderNode, RenderNodePool, RenderTarget};
use crate::render::handle::TextureHandle;
use crate::render::registry::ResourceRegistry;

pub struct RenderGraphExecutor;

impl Default for RenderGraphExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderGraphExecutor {
    pub fn new() -> Self {
        Self
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
        let target_view = match &graph.target {
            RenderTarget::Screen => surface_view.or_else(|| registry.textures.get(&TextureHandle(0))),
            RenderTarget::Offscreen { color, .. } => registry.textures.get(color),
        };

        let Some(color_view) = target_view else {
            return;
        };

        let load_op = if let Some(c) = graph.clear_color {
            wgpu::LoadOp::Clear(wgpu::Color {
                r: c[0] as f64,
                g: c[1] as f64,
                b: c[2] as f64,
                a: c[3] as f64,
            })
        } else {
            wgpu::LoadOp::Load
        };

        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view: color_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: load_op,
                store: wgpu::StoreOp::Store,
            },
        })];

        let depth_stencil_attachment = graph.depth_stencil.and_then(|handle| {
            registry.textures.get(&handle).map(|view| wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: if graph.clear_color.is_some() {
                        wgpu::LoadOp::Clear(1.0)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            })
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

        for &node_id in &graph.node_ids {
            let Some(node) = pool.get(node_id) else { continue; };
            let commands = node.commands();

            for cmd in commands {
                if current_pipeline != Some(cmd.pipeline) {
                    if let Some(pipe) = registry.pipelines.get(&cmd.pipeline) {
                        render_pass.set_pipeline(pipe);
                        current_pipeline = Some(cmd.pipeline);
                    } else {
                        continue;
                    }
                }

                for (slot, bg_handle, offsets) in &cmd.bind_groups {
                    if let Some(bg) = registry.bind_groups.get(bg_handle) {
                        render_pass.set_bind_group(*slot, bg, offsets);
                    }
                }

                match &cmd.action {
                    DrawAction::Indexed {
                        mesh,
                        index_range,
                        instance_range,
                    } => {
                        if let Some((vbo, ibo_info, _count)) = registry.meshes.get(mesh) {
                            render_pass.set_vertex_buffer(0, vbo.slice(..));
                            if let Some((ibo, format)) = ibo_info {
                                render_pass.set_index_buffer(ibo.slice(..), *format);
                                render_pass.draw_indexed(index_range.clone(), 0, instance_range.clone());
                            } else {
                                render_pass.draw(index_range.clone(), instance_range.clone());
                            }
                        }
                    }

                    DrawAction::Procedural {
                        vertex_count,
                        instance_range,
                    } => {
                        render_pass.draw(0..*vertex_count, instance_range.clone());
                    }
                }
            }
        }
    }
}
