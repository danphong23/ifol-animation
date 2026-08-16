use criterion::{criterion_group, criterion_main, Criterion};
use ifol_gpu::backend::GpuEngineBuilder;
use ifol_gpu::graph::{ComputeCommand, RenderGraph, RenderNodePool, RenderTarget};
use ifol_gpu::resources::{ResourceRegistry, BufferResourceDescriptor, BindGroupResourceDescriptor, PipelineLayoutResourceDescriptor};
use ifol_gpu::resources::handle::{BufferHandle, ComputePipelineHandle, BindGroupHandle};
use ifol_gpu::execution::RenderGraphExecutor;

fn bench_compute_1m_particles(c: &mut Criterion) {
    let mut group = c.benchmark_group("Compute Engine Benchmarks");

    pollster::block_on(async {
        let engine = GpuEngineBuilder::new()
            .with_backends(wgpu::Backends::PRIMARY)
            .with_required_limits(wgpu::Limits::default())
            .build()
            .await
            .expect("Failed to create GpuEngine for benchmark");

        let mut registry = ResourceRegistry::new();
        let mut executor = RenderGraphExecutor::new();

        // Prepare 1M particle buffer (32MB)
        let particle_count = 1_000_000;
        let buf_size = particle_count * 32;
        let buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bench 1M Buffer"),
            size: buf_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let buf_h = BufferHandle(1);
        registry.insert_buffer_with_descriptor(
            buf_h,
            buffer,
            BufferResourceDescriptor {
                size: buf_size as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            },
        ).unwrap();

        let compute_bgl = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bench_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let shader_path = std::path::Path::new(manifest_dir)
            .join("tests").join("shared_assets").join("shaders").join("compute_1m_particles.wgsl");
        let shader_code = std::fs::read_to_string(&shader_path).unwrap();
        let shader_module = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_1m_particles.wgsl"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&shader_code)),
        });

        let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bench_layout"),
            bind_group_layouts: &[Some(&compute_bgl)],
            immediate_size: 0,
        });

        let pipeline = engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("bench_pipeline"),
            layout: Some(&layout),
            module: &shader_module,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let pipe_h = ComputePipelineHandle(1);
        registry.insert_compute_pipeline_with_layout_descriptor(
            pipe_h,
            pipeline,
            PipelineLayoutResourceDescriptor {
                bind_group_layout_signatures: vec![Some(1)],
            },
        );

        let bg = {
            let raw_b = registry.buffer(&buf_h).unwrap();
            engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bench_bg"),
                layout: &compute_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: raw_b.as_entire_binding() },
                ],
            })
        };

        let bg_h = BindGroupHandle(1);
        registry.insert_bind_group_with_descriptor(
            bg_h,
            bg,
            BindGroupResourceDescriptor {
                dynamic_offset_count: 0,
                dynamic_offset_alignment: 0,
                layout_signature: 1,
            },
        ).unwrap();

        group.bench_function("1M Particle Physics Compute Dispatch", |b| {
            b.iter(|| {
                let mut pool = RenderNodePool::new();
                let mut graph = RenderGraph::new(RenderTarget::Screen);

                graph.add_compute_batch(&mut pool, vec![
                    ComputeCommand::new(pipe_h, [15625, 1, 1])
                        .with_bind_group(0, bg_h, Vec::new()),
                ]);

                let sub = executor.execute(&engine, &registry, &mut pool, &graph).unwrap();
                engine.device().poll(wgpu::PollType::Wait {
                    submission_index: Some(sub),
                    timeout: None,
                });
            });
        });
    });

    group.finish();
}

criterion_group!(benches, bench_compute_1m_particles);
criterion_main!(benches);
