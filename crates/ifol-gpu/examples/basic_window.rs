use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};
use ifol_gpu::api::builder::GpuEngineBuilder;
use ifol_gpu::api::engine::GpuEngine;
use ifol_gpu::render::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use ifol_gpu::render::compiler::RenderGraphExecutor;
use ifol_gpu::render::handle::{BindGroupHandle, PipelineHandle};
use ifol_gpu::render::registry::ResourceRegistry;

struct App<'a> {
    window: Option<Arc<Window>>,
    engine: Option<GpuEngine<'a>>,
    executor: RenderGraphExecutor,
    registry: ResourceRegistry,
    pool: ifol_gpu::render::RenderNodePool,
}

impl<'a> Default for App<'a> {
    fn default() -> Self {
        Self {
            window: None,
            engine: None,
            executor: RenderGraphExecutor::new(),
            registry: ResourceRegistry::new(),
            pool: ifol_gpu::render::RenderNodePool::new(),
        }
    }
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("iFol GPU Basic Window Test")
            .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 1024.0));
        let window = Arc::new(event_loop.create_window(attributes).unwrap());
        self.window = Some(window.clone());

        let builder = GpuEngineBuilder::new();
        let surface = builder.instance().create_surface(window).unwrap();
        let engine = pollster::block_on(builder.with_surface(surface).build()).unwrap();

        // Load Image
        let img_data = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/ai_demo_large.png")).unwrap();
        let img = image::load_from_memory(&img_data).unwrap().to_rgba8();
        let dims = img.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dims.0,
            height: dims.1,
            depth_or_array_layers: 1,
        };

        let src_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("diffuse_texture"),
            view_formats: &[],
        });

        engine.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &src_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &img,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dims.0),
                rows_per_image: Some(dims.1),
            },
            texture_size,
        );

        let src_view = src_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let src_sampler = engine.device().create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                }
            ],
            label: Some("texture_bind_group_layout"),
        });

        let bind_group = engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&src_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&src_sampler),
                }
            ],
            label: Some("diffuse_bind_group"),
        });
        self.registry.insert_bind_group(BindGroupHandle(1), bind_group);

        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/basic_texture.wgsl")).unwrap().into()),
        });

        let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let swapchain_format = engine.surface_format().unwrap();

        let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: swapchain_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });
        self.registry.insert_pipeline(PipelineHandle(1), pipeline);

        self.engine = Some(engine);
        
        // Yêu cầu vẽ ngay sau khi khởi tạo xong
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if let Some(engine) = &self.engine {
            if let Some(window) = &self.window {
                if window_id == window.id() {
                    match event {
                        WindowEvent::CloseRequested => {
                            event_loop.exit();
                        }
                        WindowEvent::Resized(physical_size) => {
                            engine.resize_surface(physical_size.width, physical_size.height);
                            window.request_redraw();
                        }
                        WindowEvent::RedrawRequested => {
                            if let Some(surface) = engine.surface() {
                                let frame = match surface.get_current_texture() {
                                    wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                                    wgpu::CurrentSurfaceTexture::Timeout => { log::warn!("Surface Timeout"); return; }
                                    wgpu::CurrentSurfaceTexture::Outdated => { log::warn!("Surface Outdated"); return; }
                                    wgpu::CurrentSurfaceTexture::Lost => { log::warn!("Surface Lost"); return; }
                                    _ => return,
                                };
                                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

                                let mut graph = RenderGraph::new(RenderTarget::Screen)
                                    .with_clear_color([0.1, 0.2, 0.3, 1.0]);

                                let cmd = DrawCommand::new(
                                    PipelineHandle(1),
                                    DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 },
                                ).with_bind_group(0, BindGroupHandle(1), vec![]);

                                graph.add_batch(&mut self.pool, vec![cmd]);

                                let idx = self.executor.execute_with_surface(engine, &self.registry, &mut self.pool, &graph, Some(&view));
                                let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
                                engine.queue().present(frame);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
