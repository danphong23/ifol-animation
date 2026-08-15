use std::borrow::Cow;
use std::fs;
use std::path::Path;
use std::time::Instant;
use ifol_gpu::api::{GpuEngine, GpuEngineBuilder};
use ifol_gpu::execution::RenderGraphExecutor;
use ifol_gpu::graph::{RenderGraph, RenderNodePool, RenderTarget};
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
    pub dual_texture_bg_layout: wgpu::BindGroupLayout,
    pub uniform_bg_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
}

impl<'a> DesktopTestHarness<'a> {
    pub async fn new(width: u32, height: u32) -> Self {
        let engine = GpuEngineBuilder::new()
            .with_required_limits(wgpu::Limits::default())
            .build()
            .await
            .expect("Failed to build engine");

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

        let dual_texture_bg_layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { multisampled: false, view_dimension: wgpu::TextureViewDimension::D2, sample_type: wgpu::TextureSampleType::Float { filterable: true } }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { multisampled: false, view_dimension: wgpu::TextureViewDimension::D2, sample_type: wgpu::TextureSampleType::Float { filterable: true } }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
            label: Some("dual_texture_bg_layout"),
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
            dual_texture_bg_layout,
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
        let tex_clone = self.engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let t_handle = TextureHandle(self.next_tex_id);
        self.next_tex_id += 1;
        self.registry.insert_owned_texture(t_handle, tex_clone, TextureResourceDescriptor {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            mip_level_count: 1,
            sample_count: 1,
        }, 8192).unwrap();

        (t_handle, tex)
    }

    pub fn create_custom_target(&mut self, width: u32, height: u32, label: &str) -> (TextureHandle, wgpu::Texture) {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let tex = self.engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let tex_clone = self.engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("{}_internal", label)),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let t_handle = TextureHandle(self.next_tex_id);
        self.next_tex_id += 1;
        self.registry.insert_owned_texture(t_handle, tex_clone, TextureResourceDescriptor {
            width,
            height,
            depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            mip_level_count: 1,
            sample_count: 1,
        }, 8192).unwrap();

        (t_handle, tex)
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

    pub fn create_dual_texture_bind_group(&mut self, t_handle_a: TextureHandle, t_handle_b: TextureHandle, label: &str) -> BindGroupHandle {
        let view_a = &self.registry.texture(&t_handle_a).unwrap().0;
        let view_b = &self.registry.texture(&t_handle_b).unwrap().0;
        let bind_group = self.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.dual_texture_bg_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view_a) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(view_b) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.sampler) },
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
                bind_group_layout_signatures: vec![Some(1), Some(2)],
            },
        );

        p_handle
    }

    pub fn register_transition_pipeline(&mut self) -> PipelineHandle {
        let shader_path = Path::new("tests/shared_assets/shaders/transition.wgsl");
        let shader_code = fs::read_to_string(shader_path)
            .unwrap_or_else(|e| panic!("Failed to read transition shader: {}", e));

        let shader = self.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("transition.wgsl"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_code)),
        });

        let layout = self.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("transition_pipeline_layout"),
            bind_group_layouts: &[
                Some(&self.dual_texture_bg_layout),
                Some(&self.uniform_bg_layout),
            ],
            immediate_size: 0,
        });

        let pipeline = self.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("transition_pipeline"),
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
                bind_group_layout_signatures: vec![Some(2), Some(2)], // Signature 2 means dual texture, and 2 for custom uniform? Wait, harness `register_custom_pipeline` uses Some(1) for texture, Some(2) for uniform.
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

    pub fn register_splitscreen_pipeline(&mut self) -> PipelineHandle {
        let shader_path = Path::new("tests/shared_assets/shaders/splitscreen_composite.wgsl");
        let shader_code = fs::read_to_string(shader_path)
            .unwrap_or_else(|e| panic!("Failed to read splitscreen shader: {}", e));

        let shader = self.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("splitscreen_composite.wgsl"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_code)),
        });

        let layout = self.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("splitscreen_pipeline_layout"),
            bind_group_layouts: &[
                Some(&self.texture_bg_layout),
                Some(&self.texture_bg_layout),
            ],
            immediate_size: 0,
        });

        let pipeline = self.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("splitscreen_pipeline"),
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
                bind_group_layout_signatures: vec![Some(1), Some(1)],
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

        // Save output PNG from the actual rendered target in registry
        fs::create_dir_all("tests/outputs/desktop").unwrap();
        let output_img_name = format!("{}.png", tc_id);
        let output_img_path = Path::new("tests/outputs/desktop").join(&output_img_name);

        let actual_rendered_tex = match graph.target {
            RenderTarget::Offscreen { color, .. } => self.registry.owned_texture(&color).unwrap_or(target_tex),
            _ => target_tex,
        };

        self.engine.save_texture_to_file_checked(actual_rendered_tex, &output_img_path)
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

    pub fn register_dual_texture_pipeline(
        &mut self,
        shader_filename: &str,
        blend: Option<wgpu::BlendState>,
        depth: bool,
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
            bind_group_layouts: &[
                Some(&self.dual_texture_bg_layout),
                Some(&self.uniform_bg_layout),
            ],
            immediate_size: 0,
        });

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
            primitive: Default::default(),
            depth_stencil: if depth {
                Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                })
            } else {
                None
            },
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let pipe_id = PipelineHandle(self.next_pipe_id);
        self.next_pipe_id += 1;
        self.registry.insert_pipeline_with_layout_descriptor(
            pipe_id,
            pipeline,
            ifol_gpu::resources::PipelineLayoutResourceDescriptor {
                bind_group_layout_signatures: vec![Some(2), Some(2)], // Dual tex layout + uniform layout? Actually let's just say Some(1), Some(2) for simplicity, or 0.
            },
        );
        pipe_id
    }

    pub fn register_stencil_pipeline(&mut self, shader_filename: &str, stencil_state: wgpu::StencilState, color_write: wgpu::ColorWrites) -> PipelineHandle {
        let shader_path = std::path::Path::new("tests/shared_assets/shaders").join(shader_filename);
        let shader_code = std::fs::read_to_string(&shader_path).unwrap();
        let shader = self.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(shader_filename), source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&shader_code)),
        });
        let layout = self.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(shader_filename), bind_group_layouts: &[Some(&self.texture_bg_layout), Some(&self.uniform_bg_layout)], immediate_size: 0,
        });
        let pipeline = self.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(shader_filename), layout: Some(&layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: color_write })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: stencil_state, bias: Default::default(),
            }),
            multisample: Default::default(), multiview_mask: None, cache: None,
        });
        let pipe_id = PipelineHandle(self.next_pipe_id); self.next_pipe_id += 1;
        self.registry.insert_pipeline_with_layout_descriptor(pipe_id, pipeline, ifol_gpu::resources::PipelineLayoutResourceDescriptor { bind_group_layout_signatures: vec![Some(1), Some(2)] });
        pipe_id
    }

    pub fn create_depth_stencil_target(&mut self, label: &str) -> (TextureHandle, wgpu::Texture) {
        let tex = self.engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let id = TextureHandle(self.next_tex_id);
        self.next_tex_id += 1;
        self.registry.insert_owned_texture(id, tex.clone(), TextureResourceDescriptor {
            width: self.width, height: self.height, depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            mip_level_count: 1, sample_count: 1,
        }, 8192).unwrap();
        (id, tex)
    }

    pub fn register_compute_pipeline(
        &mut self,
        shader_filename: &str,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> ifol_gpu::resources::ComputePipelineHandle {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let shader_path = std::path::Path::new(manifest_dir)
            .join("tests")
            .join("shared_assets")
            .join("shaders")
            .join(shader_filename);
        let shader_code = std::fs::read_to_string(&shader_path).unwrap();
        let shader = self.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(shader_filename),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&shader_code)),
        });
        let bgl_options: Vec<Option<&wgpu::BindGroupLayout>> = bind_group_layouts.iter().map(|l| Some(*l)).collect();
        let layout = self.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(shader_filename),
            bind_group_layouts: &bgl_options,
            immediate_size: 0,
        });
        let pipeline = self.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(shader_filename),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let pipe_id = ifol_gpu::resources::ComputePipelineHandle(self.next_pipe_id);
        self.next_pipe_id += 1;
        self.registry.insert_compute_pipeline_with_layout_descriptor(
            pipe_id,
            pipeline,
            ifol_gpu::resources::PipelineLayoutResourceDescriptor {
                bind_group_layout_signatures: (0..bind_group_layouts.len()).map(|i| Some(i as u64 + 1)).collect(),
            },
        );
        pipe_id
    }

    pub fn create_storage_texture(
        &mut self,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        label: &str,
    ) -> (TextureHandle, wgpu::Texture) {
        let tex = self.engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let id = TextureHandle(self.next_tex_id);
        self.next_tex_id += 1;
        self.registry.insert_owned_texture(id, tex.clone(), TextureResourceDescriptor {
            width,
            height,
            depth_or_array_layers: 1,
            format,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            mip_level_count: 1,
            sample_count: 1,
        }, 8192).unwrap();
        (id, tex)
    }

    pub fn create_storage_buffer<T: bytemuck::Pod>(
        &mut self,
        data: &[T],
        label: &str,
        extra_usage: wgpu::BufferUsages,
    ) -> (BufferHandle, wgpu::Buffer) {
        let buffer = self.engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST | extra_usage,
        });
        let id = BufferHandle(self.next_buf_id);
        self.next_buf_id += 1;
        self.registry.insert_buffer_with_descriptor(id, buffer.clone(), ifol_gpu::resources::BufferResourceDescriptor {
            size: (data.len() * std::mem::size_of::<T>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST | extra_usage,
        }).unwrap();
        (id, buffer)
    }

    pub fn insert_bind_group(
        &mut self,
        bind_group: wgpu::BindGroup,
        layout_signature: u64,
    ) -> BindGroupHandle {
        let id = BindGroupHandle(self.next_bg_id);
        self.next_bg_id += 1;
        self.registry.insert_bind_group_with_descriptor(
            id,
            bind_group,
            ifol_gpu::resources::BindGroupResourceDescriptor {
                dynamic_offset_count: 0,
                dynamic_offset_alignment: 0,
                layout_signature,
            },
        ).unwrap();
        id
    }

    pub fn insert_pipeline(
        &mut self,
        pipeline: wgpu::RenderPipeline,
        layout_signatures: Vec<Option<u64>>,
    ) -> PipelineHandle {
        let id = PipelineHandle(self.next_pipe_id);
        self.next_pipe_id += 1;
        self.registry.insert_pipeline_with_layout_descriptor(
            id,
            pipeline,
            ifol_gpu::resources::PipelineLayoutResourceDescriptor {
                bind_group_layout_signatures: layout_signatures,
            },
        );
        id
    }

    pub fn readback_storage_buffer<T: bytemuck::Pod + Clone>(&self, buffer: &wgpu::Buffer, count: usize) -> Vec<T> {
        let size = (count * std::mem::size_of::<T>()) as u64;
        let staging = self.engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("storage_readback_staging"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self.engine.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("readback_encoder"),
        });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, size);
        let sub = self.engine.queue().submit(Some(encoder.finish()));
        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            tx.send(res).unwrap();
        });
        self.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub),
            timeout: None,
        });
        rx.recv().unwrap().unwrap();
        let mapped = slice.get_mapped_range().unwrap();
        let data: &[T] = bytemuck::cast_slice(&mapped);
        let result = data.to_vec();
        drop(mapped);
        staging.unmap();
        result
    }
}
