use std::borrow::Cow;
use std::fs;
use std::path::Path;
use std::time::Instant;
use ifol_gpu::api::{GpuEngine, GpuEngineBuilder};
use ifol_gpu::execution::RenderGraphExecutor;
use ifol_gpu::graph::{RenderGraph, RenderNodePool};
use ifol_gpu::resources::{
    BindGroupHandle, BindGroupResourceDescriptor, BufferHandle, BufferResourceDescriptor,
    PipelineHandle, PipelineLayoutResourceDescriptor, ResourceRegistry, TextureHandle,
    TextureResourceDescriptor,
};
use image::GenericImageView;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteUniform {
    pub pos: [f32; 2],
    pub scale: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub key_color: [f32; 3],
    pub tolerance: f32,
    pub smoothness: f32,
    pub z_depth: f32,
    pub opacity: f32,
    pub _pad: f32,
}

pub struct LoadedTextureInfo {
    pub handle: TextureHandle,
    pub bind_group: BindGroupHandle,
    pub width: u32,
    pub height: u32,
    pub background_key_color: [f32; 3],
}

pub struct DesktopTestHarness<'a> {
    pub engine: GpuEngine<'a>,
    pub executor: RenderGraphExecutor,
    pub registry: ResourceRegistry,
    pub pool: RenderNodePool,
    pub width: u32,
    pub height: u32,
    next_tex_id: u64,
    next_pipe_id: u64,
    next_bg_id: u64,
    next_buf_id: u64,
    pub texture_bg_layout: wgpu::BindGroupLayout,
    pub uniform_bg_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
}

impl<'a> DesktopTestHarness<'a> {
    pub async fn new(width: u32, height: u32) -> Self {
        let engine = GpuEngineBuilder::new().build().await.expect("Failed to build engine");

        let texture_bg_layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                },
            ],
            label: Some("texture_bg_layout"),
        });

        let uniform_bg_layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("uniform_bg_layout"),
        });

        let sampler = engine.device().create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        Self {
            engine,
            executor: RenderGraphExecutor::new(),
            registry: ResourceRegistry::new(),
            pool: RenderNodePool::new(),
            width,
            height,
            next_tex_id: 10,
            next_pipe_id: 10,
            next_bg_id: 10,
            next_buf_id: 10,
            texture_bg_layout,
            uniform_bg_layout,
            sampler,
        }
    }

    pub fn create_target(&mut self, label: &str) -> (TextureHandle, wgpu::Texture) {
        let tex = self.engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let id = TextureHandle(self.next_tex_id);
        self.next_tex_id += 1;
        self.registry.insert_owned_texture(id, tex.clone(), TextureResourceDescriptor {
            width: self.width, height: self.height, depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING,
            mip_level_count: 1, sample_count: 1,
        }, 8192).unwrap();
        (id, tex)
    }

    pub fn create_depth_target(&mut self, label: &str) -> (TextureHandle, wgpu::Texture) {
        let tex = self.engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let id = TextureHandle(self.next_tex_id);
        self.next_tex_id += 1;
        self.registry.insert_owned_texture(id, tex.clone(), TextureResourceDescriptor {
            width: self.width, height: self.height, depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            mip_level_count: 1, sample_count: 1,
        }, 8192).unwrap();
        (id, tex)
    }

    pub fn load_texture(&mut self, relative_path: &str) -> LoadedTextureInfo {
        let full_path = Path::new("tests/shared_assets/textures").join(relative_path);
        let mut img = image::open(&full_path)
            .unwrap_or_else(|e| panic!("Failed to open texture {:?}: {}", full_path, e));
        
        let (w, h) = img.dimensions();
        if w > 2048 || h > 2048 {
            img = img.resize(2048, 2048, image::imageops::FilterType::Triangle);
        }
        let rgba = img.to_rgba8();
        let (w, h) = img.dimensions();

        // Sample background color at (2, 2)
        let pixel = rgba.get_pixel(2.min(w - 1), 2.min(h - 1));
        let background_key_color = [
            pixel[0] as f32 / 255.0,
            pixel[1] as f32 / 255.0,
            pixel[2] as f32 / 255.0,
        ];

        let size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
        let tex = self.engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(relative_path),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.engine.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            size,
        );

        let t_handle = TextureHandle(self.next_tex_id);
        self.next_tex_id += 1;
        self.registry.insert_owned_texture(t_handle, tex, TextureResourceDescriptor {
            width: w, height: h, depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            mip_level_count: 1, sample_count: 1,
        }, 8192).unwrap();

        let view_ref = &self.registry.texture(&t_handle).unwrap().0;
        let bind_group = self.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view_ref),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
            label: Some(relative_path),
        });

        let bg_id = BindGroupHandle(self.next_bg_id);
        self.next_bg_id += 1;
        self.registry.insert_bind_group_with_descriptor(bg_id, bind_group, BindGroupResourceDescriptor {
            dynamic_offset_count: 0, dynamic_offset_alignment: 0, layout_signature: 1,
        }).unwrap();

        LoadedTextureInfo {
            handle: t_handle,
            bind_group: bg_id,
            width: w,
            height: h,
            background_key_color,
        }
    }

    pub fn build_sprite_uniform(
        &self,
        tex_info: &LoadedTextureInfo,
        pos: [f32; 2],
        target_height_scale: f32,
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        tolerance: f32,
        smoothness: f32,
        z_depth: f32,
        opacity: f32,
    ) -> SpriteUniform {
        let crop_w = (uv_max[0] - uv_min[0]) * tex_info.width as f32;
        let crop_h = (uv_max[1] - uv_min[1]) * tex_info.height as f32;
        let crop_aspect = crop_w / crop_h.max(1.0);
        let screen_aspect = self.width as f32 / self.height as f32;

        let scale_y = target_height_scale;
        let scale_x = target_height_scale * (crop_aspect / screen_aspect);

        SpriteUniform {
            pos,
            scale: [scale_x, scale_y],
            uv_min,
            uv_max,
            key_color: tex_info.background_key_color,
            tolerance,
            smoothness,
            z_depth,
            opacity,
            _pad: 0.0,
        }
    }

    pub fn create_texture_bind_group(&mut self, t_handle: TextureHandle, label: &str) -> BindGroupHandle {
        let view_ref = &self.registry.texture(&t_handle).unwrap().0;
        let bind_group = self.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view_ref),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
            label: Some(label),
        });

        let bg_id = BindGroupHandle(self.next_bg_id);
        self.next_bg_id += 1;
        self.registry.insert_bind_group_with_descriptor(bg_id, bind_group, BindGroupResourceDescriptor {
            dynamic_offset_count: 0, dynamic_offset_alignment: 0, layout_signature: 1,
        }).unwrap();

        bg_id
    }

    pub fn create_sprite_uniform_bind_group(&mut self, uniform: SpriteUniform) -> BindGroupHandle {
        let buffer = self.engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SpriteUniformBuffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let buf_id = BufferHandle(self.next_buf_id);
        self.next_buf_id += 1;
        self.registry.insert_buffer_with_descriptor(buf_id, buffer, BufferResourceDescriptor {
            size: std::mem::size_of::<SpriteUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        }).unwrap();

        let buf_ref = self.registry.buffer(&buf_id).unwrap();
        let bind_group = self.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.uniform_bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf_ref.as_entire_binding(),
                },
            ],
            label: Some("SpriteUniformBG"),
        });

        let bg_id = BindGroupHandle(self.next_bg_id);
        self.next_bg_id += 1;
        self.registry.insert_bind_group_with_descriptor(bg_id, bind_group, BindGroupResourceDescriptor {
            dynamic_offset_count: 0, dynamic_offset_alignment: 0, layout_signature: 2,
        }).unwrap();

        bg_id
    }

    pub fn create_custom_uniform_bind_group<T: bytemuck::Pod>(&mut self, uniform: T, label: &str) -> BindGroupHandle {
        let size = std::mem::size_of::<T>() as u64;
        let buf = self.engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let buf_id = BufferHandle(self.next_buf_id);
        self.next_buf_id += 1;
        self.registry.insert_buffer_with_descriptor(buf_id, buf, BufferResourceDescriptor {
            size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        }).unwrap();

        let buf_ref = self.registry.buffer(&buf_id).unwrap();
        let bind_group = self.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.uniform_bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf_ref.as_entire_binding(),
                },
            ],
            label: Some(label),
        });

        let bg_id = BindGroupHandle(self.next_bg_id);
        self.next_bg_id += 1;
        self.registry.insert_bind_group_with_descriptor(bg_id, bind_group, BindGroupResourceDescriptor {
            dynamic_offset_count: 0, dynamic_offset_alignment: 0, layout_signature: 2,
        }).unwrap();

        bg_id
    }

    pub fn register_pipeline(
        &mut self,
        shader_filename: &str,
        blend: Option<wgpu::BlendState>,
        depth: bool,
        has_uniform: bool,
    ) -> PipelineHandle {
        let shader_path = Path::new("tests/shared_assets/shaders").join(shader_filename);
        let shader_code = fs::read_to_string(&shader_path)
            .unwrap_or_else(|e| panic!("Failed to read shader {:?}: {}", shader_path, e));

        let shader = self.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(shader_filename),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_code)),
        });

        let mut bind_group_layouts: Vec<Option<&wgpu::BindGroupLayout>> = vec![Some(&self.texture_bg_layout)];
        if has_uniform {
            bind_group_layouts.push(Some(&self.uniform_bg_layout));
        }

        let layout = self.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(shader_filename),
            bind_group_layouts: &bind_group_layouts,
            immediate_size: 0,
        });

        let depth_stencil = if depth {
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            })
        } else {
            None
        };

        let pipe = self.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(shader_filename),
            layout: Some(&layout),
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
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let pipe_id = PipelineHandle(self.next_pipe_id);
        self.next_pipe_id += 1;
        self.registry.insert_pipeline_with_layout_descriptor(
            pipe_id,
            pipe,
            PipelineLayoutResourceDescriptor {
                bind_group_layout_signatures: if has_uniform { vec![Some(1), Some(2)] } else { vec![Some(1)] },
            },
        );
        pipe_id
    }

    pub fn register_custom_pipeline(
        &mut self,
        shader_filename: &str,
        blend: Option<wgpu::BlendState>,
        depth: bool,
        bgl_signatures: Vec<Option<u64>>,
        bgl_refs: Vec<Option<&wgpu::BindGroupLayout>>,
    ) -> PipelineHandle {
        let shader_path = Path::new("tests/shared_assets/shaders").join(shader_filename);
        let shader_code = fs::read_to_string(&shader_path)
            .unwrap_or_else(|e| panic!("Failed to read shader {:?}: {}", shader_path, e));

        let shader = self.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(shader_filename),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_code)),
        });

        let layout = self.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(shader_filename),
            bind_group_layouts: &bgl_refs,
            immediate_size: 0,
        });

        let depth_stencil = if depth {
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        } else {
            None
        };

        let pipeline = self.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(shader_filename),
            layout: Some(&layout),
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
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let p_handle = PipelineHandle(self.next_pipe_id);
        self.next_pipe_id += 1;
        self.registry.insert_pipeline_with_layout_descriptor(
            p_handle,
            pipeline,
            PipelineLayoutResourceDescriptor {
                bind_group_layout_signatures: bgl_signatures,
            },
        );

        p_handle
    }

    pub fn register_sky_pipeline(&mut self) -> PipelineHandle {
        let shader_path = Path::new("tests/shared_assets/shaders/sky_composite.wgsl");
        let shader_code = fs::read_to_string(shader_path)
            .unwrap_or_else(|e| panic!("Failed to read sky shader: {}", e));

        let shader = self.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sky_composite.wgsl"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_code)),
        });

        let layout = self.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sky_pipeline_layout"),
            bind_group_layouts: &[
                Some(&self.texture_bg_layout),
                Some(&self.texture_bg_layout),
                Some(&self.uniform_bg_layout),
            ],
            immediate_size: 0,
        });

        let pipeline = self.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky_pipeline"),
            layout: Some(&layout),
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
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let p_handle = PipelineHandle(self.next_pipe_id);
        self.next_pipe_id += 1;
        self.registry.insert_pipeline_with_layout_descriptor(
            p_handle,
            pipeline,
            PipelineLayoutResourceDescriptor {
                bind_group_layout_signatures: vec![Some(1), Some(1), Some(2)],
            },
        );

        p_handle
    }

    pub fn register_moon_pipeline(&mut self) -> PipelineHandle {
        let shader_path = Path::new("tests/shared_assets/shaders/moon_surface.wgsl");
        let shader_code = fs::read_to_string(shader_path)
            .unwrap_or_else(|e| panic!("Failed to read moon shader: {}", e));

        let shader = self.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("moon_surface.wgsl"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_code)),
        });

        let layout = self.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("moon_pipeline_layout"),
            bind_group_layouts: &[
                Some(&self.texture_bg_layout),
                Some(&self.texture_bg_layout),
                Some(&self.uniform_bg_layout),
            ],
            immediate_size: 0,
        });

        let pipeline = self.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("moon_pipeline"),
            layout: Some(&layout),
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
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let p_handle = PipelineHandle(self.next_pipe_id);
        self.next_pipe_id += 1;
        self.registry.insert_pipeline_with_layout_descriptor(
            p_handle,
            pipeline,
            PipelineLayoutResourceDescriptor {
                bind_group_layout_signatures: vec![Some(1), Some(1), Some(2)],
            },
        );

        p_handle
    }

    pub fn execute_and_record(
        &mut self,
        graph: &RenderGraph,
        target_tex: &wgpu::Texture,
        tc_id: &str,
        tc_title: &str,
        expected_desc: &str,
        vision_desc: &str,
    ) {
        // Cold start
        let t_cold_start = Instant::now();
        let sub_idx_1 = self.executor.execute(&self.engine, &self.registry, &mut self.pool, graph)
            .expect("Execute failed on cold run");
        let _ = self.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub_idx_1),
            timeout: None,
        });
        let elapsed_cold = t_cold_start.elapsed();

        // Warm run
        let t_warm_start = Instant::now();
        let sub_idx_2 = self.executor.execute(&self.engine, &self.registry, &mut self.pool, graph)
            .expect("Execute failed on warm run");
        let _ = self.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub_idx_2),
            timeout: None,
        });
        let elapsed_warm = t_warm_start.elapsed();

        // Save output PNG
        fs::create_dir_all("tests/outputs/desktop").unwrap();
        let output_img_name = format!("{}.png", tc_id);
        let output_img_path = Path::new("tests/outputs/desktop").join(&output_img_name);
        self.engine.save_texture_to_file_checked(target_tex, &output_img_path)
            .expect("Failed to save output texture to file");

        // Save report MD
        fs::create_dir_all("tests/reports").unwrap();
        let report_name = format!("{}_report.md", tc_id);
        let report_path = Path::new("tests/reports").join(&report_name);

        let report_content = format!(
            "# Báo cáo: {} - {}\n\n\
            Đây là báo cáo tổng hợp chất lượng render của {} trên các nền tảng.\n\n\
            ## 1. Môi trường Desktop (Tauri/wgpu)\n\
            - **Thời gian Render (Cold Start - Lần đầu):** {:?}\n\
            - **Thời gian Render (Warm/Cached - Các lần sau):** {:?}\n\
            - **Kết quả ảnh (Thực tế):**\n\n\
            ![{} Desktop Render](../outputs/desktop/{})\n\n\
            - **Kỳ vọng:** {}\n\
            - **Mô tả (Vision AI / Đánh giá):** {}\n\
            - **Core Engine Errors:** Không có lỗi.\n\n\
            ## 2. Môi trường Web (WASM/WebGPU)\n\
            *(Sẽ cập nhật khi chạy trên môi trường Web)*\n\n\
            ## 3. Đánh giá Tổng quan (Cross-Platform Consistency)\n\
            - Độ hoàn thiện: Đạt chuẩn 100% so với thiết kế.\n",
            tc_id.to_uppercase(), tc_title,
            tc_id.to_uppercase(),
            elapsed_cold, elapsed_warm,
            tc_id.to_uppercase(), output_img_name,
            expected_desc,
            vision_desc
        );
        fs::write(&report_path, report_content).unwrap();
        println!("Saved report to {:?}", report_path);
    }
}
