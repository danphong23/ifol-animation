use crate::api::GpuEngine;
use crate::render::{DrawCommand, RenderGraph};
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
    pub fn execute(&self, engine: &GpuEngine, registry: &ResourceRegistry, graph: &RenderGraph) -> wgpu::SubmissionIndex {
        let mut encoder = engine.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("RenderGraphEncoder"),
        });

        for node in &graph.nodes {
            let mut color_attachments = Vec::new();
            
            // Móc nối (Resolve) các TextureHandle thành wgpu::TextureView từ Registry
            for &handle in &node.target.color_attachments {
                if let Some(view) = registry.textures.get(&handle) {
                    color_attachments.push(Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), // Hiện tại giả lập Clear, sau sẽ mở API tùy chỉnh
                            store: wgpu::StoreOp::Store,
                        },
                    }));
                }
            }

            let depth_stencil_attachment = node.target.depth_attachment.and_then(|handle| {
                registry.textures.get(&handle).map(|view| wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                })
            });

            // Bắt đầu một GPU Render Pass
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&node.name),
                color_attachments: &color_attachments,
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // Biên dịch (Duyệt) các DrawCommand do ECS gửi xuống
            for cmd in &node.commands {
                match cmd {
                    DrawCommand::DrawMesh { mesh, pipeline, bind_groups } => {
                        if let Some(pipe) = registry.pipelines.get(pipeline) {
                            render_pass.set_pipeline(pipe);
                            
                            // Bind Shader Variables (Uniforms, Textures)
                            for (i, bg_handle) in bind_groups.iter().enumerate() {
                                if let Some(bg) = registry.bind_groups.get(bg_handle) {
                                    render_pass.set_bind_group(i as u32, bg, &[]);
                                }
                            }
                            
                            // Draw Call!
                            if let Some((vbo, ibo, count)) = registry.meshes.get(mesh) {
                                render_pass.set_vertex_buffer(0, vbo.slice(..));
                                if let Some(ib) = ibo {
                                    render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);
                                    render_pass.draw_indexed(0..*count, 0, 0..1);
                                } else {
                                    render_pass.draw(0..*count, 0..1);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Submit toàn bộ khối lệnh (Command Buffer) lên card màn hình để thực thi
        engine.queue().submit(std::iter::once(encoder.finish()))
    }
}
