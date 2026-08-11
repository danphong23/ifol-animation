use crate::api::GpuEngine;
use crate::render::graph::{DrawAction, DrawCommand, RenderGraph, RenderNode, RenderTarget};
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
        graph: &RenderGraph,
    ) -> wgpu::SubmissionIndex {
        self.execute_with_surface(engine, registry, graph, None)
    }

    /// Biên dịch RenderGraph với Surface Texture View chỉ định (khi vẽ trực tiếp ra cửa sổ)
    pub fn execute_with_surface(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        graph: &RenderGraph,
        surface_view: Option<&wgpu::TextureView>,
    ) -> wgpu::SubmissionIndex {
        let mut encoder = engine.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("RenderGraphEncoder"),
        });

        // Duyệt đệ quy cây RenderGraph
        self.compile_graph(&mut encoder, graph, registry, surface_view);

        // Submit toàn bộ khối lệnh (Command Buffer) lên GPU 1 lần duy nhất
        engine.queue().submit(std::iter::once(encoder.finish()))
    }

    /// Duyệt cây đệ quy Depth-First
    fn compile_graph(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        graph: &RenderGraph,
        registry: &ResourceRegistry,
        surface_view: Option<&wgpu::TextureView>,
    ) {
        let mut first_pass_on_target = true;

        for node in &graph.nodes {
            match node {
                RenderNode::SubGraph {
                    graph: inner_graph,
                    commands,
                    ..
                } => {
                    // 1. ĐỆ QUY: Xử lý Graph con trước (vẽ ra Offscreen)
                    self.compile_graph(encoder, inner_graph, registry, surface_view);

                    // 2. Thực thi danh sách commands của SubGraph để in kết quả lên Graph cha (nếu có)
                    if !commands.is_empty() {
                        self.execute_commands_on_target(
                            encoder,
                            graph,
                            commands,
                            registry,
                            surface_view,
                            first_pass_on_target,
                        );
                        first_pass_on_target = false;
                    }
                }

                RenderNode::DrawBatch { commands, .. } => {
                    if !commands.is_empty() || graph.clear_color.is_some() {
                        self.execute_commands_on_target(
                            encoder,
                            graph,
                            commands,
                            registry,
                            surface_view,
                            first_pass_on_target,
                        );
                        first_pass_on_target = false;
                    }
                }
            }
        }
    }

    /// Mở RenderPass và thực thi một chuỗi DrawCommand lên Target của RenderGraph
    fn execute_commands_on_target(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        graph: &RenderGraph,
        commands: &[DrawCommand],
        registry: &ResourceRegistry,
        surface_view: Option<&wgpu::TextureView>,
        is_first_pass: bool,
    ) {
        // Resolve Target Texture View
        let target_view = match &graph.target {
            RenderTarget::Screen => surface_view.or_else(|| registry.textures.get(&TextureHandle(0))),
            RenderTarget::Offscreen { color, .. } => registry.textures.get(color),
        };

        let Some(color_view) = target_view else {
            // Nếu không tìm thấy Target Texture View, bỏ qua pass này an toàn
            return;
        };

        // Xác định LoadOp: Pass đầu tiên dùng clear_color (nếu có), các pass sau dùng LoadOp::Load
        let load_op = if is_first_pass {
            if let Some(c) = graph.clear_color {
                wgpu::LoadOp::Clear(wgpu::Color {
                    r: c[0] as f64,
                    g: c[1] as f64,
                    b: c[2] as f64,
                    a: c[3] as f64,
                })
            } else {
                wgpu::LoadOp::Load
            }
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

        // Resolve Depth Stencil Attachment
        let depth_stencil_attachment = graph.depth_stencil.and_then(|handle| {
            registry.textures.get(&handle).map(|view| wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: if is_first_pass {
                        wgpu::LoadOp::Clear(1.0)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            })
        });

        // Bắt đầu GPU Render Pass
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RenderPass"),
            color_attachments: &color_attachments,
            depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        // Cache trạng thái RenderPass để tối ưu hóa
        let mut current_pipeline = None;

        for cmd in commands {
            // 1. Set Pipeline
            if current_pipeline != Some(cmd.pipeline) {
                if let Some(pipe) = registry.pipelines.get(&cmd.pipeline) {
                    render_pass.set_pipeline(pipe);
                    current_pipeline = Some(cmd.pipeline);
                } else {
                    continue; // Skip nếu Pipeline không tồn tại
                }
            }

            // 2. Set Bind Groups (với dynamic offsets)
            for (slot, bg_handle, offsets) in &cmd.bind_groups {
                if let Some(bg) = registry.bind_groups.get(bg_handle) {
                    render_pass.set_bind_group(*slot, bg, offsets);
                }
            }

            // 3. Perform Draw Action
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
                            // Không có IBO -> Fallback về draw vertex theo index range
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
