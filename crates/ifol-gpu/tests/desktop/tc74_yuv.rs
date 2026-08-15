use std::time::Instant;
use ifol_gpu::graph::{ComputeCommand, RenderGraph, RenderTarget};
use image::GenericImageView;

mod harness;

fn upload_texture(device: &wgpu::Device, queue: &wgpu::Queue, data: &[u8], width: u32, height: u32, label: &str) -> wgpu::Texture {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        tex.as_image_copy(),
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );

    tex
}

#[test]
fn test_tc74_yuv() {
    pollster::block_on(async {
        let mut h = harness::DesktopTestHarness::new(800, 600).await;

        // Load an image and convert to YUV420 on CPU
        let img = image::open("tests/shared_assets/textures/bg_nightsky.jpeg").unwrap();
        let (width, height) = img.dimensions();
        // ensure even dimensions for 420
        let w = width & !1;
        let h_img = height & !1;

        let mut y_data = vec![0u8; (w * h_img) as usize];
        let mut u_data = vec![0u8; (w * h_img / 4) as usize];
        let mut v_data = vec![0u8; (w * h_img / 4) as usize];

        for y in 0..h_img {
            for x in 0..w {
                let pixel = img.get_pixel(x, y);
                let r = pixel[0] as f32 / 255.0;
                let g = pixel[1] as f32 / 255.0;
                let b = pixel[2] as f32 / 255.0;

                let y_val = 0.299 * r + 0.587 * g + 0.114 * b;
                y_data[(y * w + x) as usize] = (y_val.clamp(0.0, 1.0) * 255.0) as u8;

                if y % 2 == 0 && x % 2 == 0 {
                    let u_val = -0.168736 * r - 0.331264 * g + 0.5 * b + 0.5;
                    let v_val = 0.5 * r - 0.418688 * g - 0.081312 * b + 0.5;
                    u_data[((y / 2) * (w / 2) + (x / 2)) as usize] = (u_val.clamp(0.0, 1.0) * 255.0) as u8;
                    v_data[((y / 2) * (w / 2) + (x / 2)) as usize] = (v_val.clamp(0.0, 1.0) * 255.0) as u8;
                }
            }
        }

        let y_tex = upload_texture(h.engine.device(), h.engine.queue(), &y_data, w, h_img, "y_tex");
        let u_tex = upload_texture(h.engine.device(), h.engine.queue(), &u_data, w / 2, h_img / 2, "u_tex");
        let v_tex = upload_texture(h.engine.device(), h.engine.queue(), &v_data, w / 2, h_img / 2, "v_tex");

        let (target_handle, target_tex) = h.create_storage_texture(w, h_img, wgpu::TextureFormat::Rgba8Unorm, "tc74_target");

        let bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("yuv_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::Rgba8Unorm, view_dimension: wgpu::TextureViewDimension::D2 },
                    count: None,
                },
            ],
        });

        let y_view = y_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let u_view = u_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let v_view = v_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let out_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("yuv_bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&y_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&u_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&v_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&out_view) },
            ],
        });

        let bg_h = h.insert_bind_group(bg, 1);

        let compute_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/compute_yuv_to_rgba.wgsl").unwrap();
        let compute_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_yuv_to_rgba.wgsl"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&compute_shader_code)),
        });

        let pipeline_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("yuv_layout"), bind_group_layouts: &[Some(&bgl)], immediate_size: 0,
        });

        let pipeline = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cs_main"), layout: Some(&pipeline_layout), module: &compute_shader, entry_point: Some("cs_main"), compilation_options: Default::default(), cache: None,
        });
        
        let p_h = h.insert_compute_pipeline(pipeline, vec![Some(1)]);

        let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: target_handle, width: w, height: h_img });

        let wg_x = (w + 15) / 16;
        let wg_y = (h_img + 15) / 16;

        graph.add_compute_batch(&mut h.pool, vec![
            ComputeCommand::new(p_h, [wg_x, wg_y, 1]).with_bind_group(0, bg_h, vec![]),
        ]);
        
        let t_start = Instant::now();
        let sub = h.executor.execute(&h.engine, &h.registry, &mut h.pool, &graph).unwrap();
        let _ = h.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(sub), timeout: None });
        let t_elapsed = t_start.elapsed();
        println!("YUV to RGBA Compute Time: {:?}", t_elapsed);

        h.execute_and_record(&graph, &target_tex, "tc74_yuv", "GPU YUV 4:2:0 to RGBA", "Decode 3-plane YUV video frame back to RGB", "Render output");
    });
}
