use super::{RenderGraphExecutor, RenderGraphValidationError};
use crate::backend::GpuEngineBuilder;
use crate::graph::{ComputeCommand, RenderGraph, RenderNodePool, RenderTarget};
use crate::resources::{BindGroupHandle, ComputePipelineHandle, ResourceRegistry};

#[test]
fn validation_checks_descriptor_aware_dynamic_offsets() {
    let engine = pollster::block_on(
        GpuEngineBuilder::new()
            .with_required_limits(wgpu::Limits::default())
            .build(),
    )
    .unwrap();
    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("dynamic_offset_validation_shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
            "@group(0) @binding(0) var<uniform> value: u32; @compute @workgroup_size(1) fn main() { _ = value; }",
        )),
    });
    let layout = engine
        .device()
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dynamic_offset_validation_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
    let pipeline_layout = engine
        .device()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("dynamic_offset_validation_pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
    let pipeline = engine
        .device()
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("dynamic_offset_validation_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
    let alignment = engine.capabilities().min_uniform_buffer_offset_alignment;
    let buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("dynamic_offset_validation_buffer"),
        size: alignment as u64 * 2,
        usage: wgpu::BufferUsages::UNIFORM,
        mapped_at_creation: false,
    });
    let bind_group = engine
        .device()
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dynamic_offset_validation_bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(4),
                }),
            }],
        });
    let mismatched_bind_group = engine
        .device()
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dynamic_offset_mismatched_bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(4),
                }),
            }],
        });
    let mut registry = ResourceRegistry::new();
    registry.insert_compute_pipeline_with_layout_descriptor(
        ComputePipelineHandle(1),
        pipeline,
        crate::resources::PipelineLayoutResourceDescriptor {
            bind_group_layout_signatures: vec![Some(7)],
        },
    );
    registry
        .insert_bind_group_with_descriptor(
            BindGroupHandle(1),
            bind_group,
            crate::resources::BindGroupResourceDescriptor {
                dynamic_offset_count: 1,
                dynamic_offset_alignment: alignment,
                layout_signature: 7,
            },
        )
        .unwrap();
    registry
        .insert_bind_group_with_descriptor(
            BindGroupHandle(2),
            mismatched_bind_group,
            crate::resources::BindGroupResourceDescriptor {
                dynamic_offset_count: 1,
                dynamic_offset_alignment: alignment,
                layout_signature: 8,
            },
        )
        .unwrap();
    let mut pool = RenderNodePool::new();
    let mut valid = RenderGraph::new(RenderTarget::Screen);
    valid.add_compute_batch(
        &mut pool,
        vec![
            ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(
                0,
                BindGroupHandle(1),
                vec![alignment],
            ),
        ],
    );
    assert_eq!(
        RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &valid),
        Ok(())
    );
    let mut invalid = RenderGraph::new(RenderTarget::Screen);
    invalid.add_compute_batch(
        &mut pool,
        vec![
            ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(
                0,
                BindGroupHandle(1),
                vec![1],
            ),
        ],
    );
    assert_eq!(
        RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &invalid),
        Err(RenderGraphValidationError::InvalidDynamicOffsetAlignment {
            handle: BindGroupHandle(1),
            offset: 1,
            alignment,
        })
    );
    let mut mismatched = RenderGraph::new(RenderTarget::Screen);
    mismatched.add_compute_batch(
        &mut pool,
        vec![
            ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(
                0,
                BindGroupHandle(2),
                vec![alignment],
            ),
        ],
    );
    assert_eq!(
        RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &mismatched,),
        Err(RenderGraphValidationError::ComputePipelineLayoutMismatch {
            pipeline: ComputePipelineHandle(1),
            slot: 0,
            expected: Some(7),
            actual: Some(8),
        })
    );
}
