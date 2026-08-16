use std::time::Instant;
use ifol_gpu::graph::{ComputeCommand, RenderGraph, RenderTarget};
use wgpu::util::DeviceExt;

mod harness;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Params {
    radius: i32,
    mode: i32,
    _pad: [i32; 2],
}

#[test]
fn test_tc73_morphology() {
    pollster::block_on(async {
        let mut h = harness::DesktopTestHarness::new(800, 800).await;
        
        let (_mask_tex_h, mask_tex) = h.create_storage_texture(800, 800, wgpu::TextureFormat::Rgba8Unorm, "mask_tex");
        let (target_handle, target_tex) = h.create_storage_texture(800, 800, wgpu::TextureFormat::Rgba8Unorm, "tc73_target");
        
        let params = Params {
            radius: 10,
            mode: 0, // Dilation
            _pad: [0; 2],
        };
        let uniform_buf = h.engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Gen Mask BGL
        let gen_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gen_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });
        
        // Morph BGL
        let morph_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("morph_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });

        let mask_view = mask_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let gen_bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gen_bg"),
            layout: &gen_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&mask_view) },
            ],
        });
        
        let morph_bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("morph_bg"),
            layout: &morph_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&mask_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&target_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &uniform_buf, offset: 0, size: None }) },
            ],
        });

        let gen_bg_h = h.insert_bind_group(gen_bg, 1);
        let morph_bg_h = h.insert_bind_group(morph_bg, 2);

        let compute_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/compute_morphology.wgsl").unwrap();
        let compute_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_morphology.wgsl"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&compute_shader_code)),
        });

        let gen_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gen_layout"), bind_group_layouts: &[Some(&gen_bgl)], immediate_size: 0,
        });
        let morph_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("morph_layout"), bind_group_layouts: &[Some(&morph_bgl)], immediate_size: 0,
        });

        let p_gen = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cs_gen_mask"), layout: Some(&gen_layout), module: &compute_shader, entry_point: Some("cs_gen_mask"), compilation_options: Default::default(), cache: None,
        });
        let p_morph = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cs_main"), layout: Some(&morph_layout), module: &compute_shader, entry_point: Some("cs_main"), compilation_options: Default::default(), cache: None,
        });

        let p_gen_h = h.insert_compute_pipeline(p_gen, vec![Some(1)]);
        let p_morph_h = h.insert_compute_pipeline(p_morph, vec![Some(2)]);

        let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: target_handle, width: 800, height: 800 });

        let wg_x = (800 + 15) / 16;
        let wg_y = (800 + 15) / 16;

        graph.add_compute_batch(&mut h.pool, vec![
            ComputeCommand::new(p_gen_h, [wg_x, wg_y, 1]).with_bind_group(0, gen_bg_h, vec![]),
            ComputeCommand::new(p_morph_h, [wg_x, wg_y, 1]).with_bind_group(0, morph_bg_h, vec![]),
        ]);
        
        let t_start = Instant::now();
        let sub = h.executor.execute(&h.engine, &h.registry, &mut h.pool, &graph).unwrap();
        let _ = h.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(sub), timeout: None });
        let t_elapsed = t_start.elapsed();
        println!("Morphology Compute Time: {:?}", t_elapsed);

        h.execute_and_record(&graph, &target_tex, "tc73_morphology", "GPU Morphological Dilation", "A thin mask dilated by 10 pixels", "Render output");
    });
}
