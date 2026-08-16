use super::*;
use crate::backend::GpuEngineBuilder;
use crate::graph::{ComputeCommand, CopyCommand, DrawAction, DrawCommand};
use crate::resources::{BufferHandle, ComputePipelineHandle, PipelineHandle, ResourceRegistry};

#[test]
fn invalid_bind_group_slot_does_not_index_state_cache() {
    assert_eq!(bind_group_slot_index(0, 4), Some(0));
    assert_eq!(bind_group_slot_index(3, 4), Some(3));
    assert_eq!(bind_group_slot_index(4, 4), None);
    assert_eq!(bind_group_slot_index(7, 8), Some(7));
    assert_eq!(bind_group_slot_index(u32::MAX, 8), None);
}

#[test]
fn flat_compute_encoder_reports_missing_pipeline_instead_of_skipping() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let mut encoder = engine
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("missing-compute-pipeline"),
        });
    let command = ComputeCommand::new(ComputePipelineHandle(701), [1, 1, 1]);
    assert_eq!(
        encode_compute_commands(&mut encoder, &ResourceRegistry::new(), &[command], 4),
        Err(RenderGraphValidationError::MissingComputePipeline(
            ComputePipelineHandle(701)
        ))
    );
}

#[test]
fn flat_draw_encoder_reports_missing_pipeline_instead_of_skipping() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let mut encoder = engine
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("missing-render-pipeline"),
        });
    let view = engine
        .device()
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("missing-render-pipeline-target"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default());
    let color_attachments = [Some(wgpu::RenderPassColorAttachment {
        view: &view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: wgpu::StoreOp::Discard,
        },
    })];
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("missing-render-pipeline-pass"),
        color_attachments: &color_attachments,
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let command = DrawCommand::new(
        PipelineHandle(702),
        DrawAction::Procedural {
            vertex_count: 3,
            instance_range: 0..1,
        },
    );
    assert_eq!(
        encode_draw_commands(&mut pass, &ResourceRegistry::new(), &[command], 4),
        Err(RenderGraphValidationError::MissingPipeline(PipelineHandle(
            702
        )))
    );
}

#[test]
fn copy_encoder_reports_missing_buffer_instead_of_skipping() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let mut encoder = engine
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("missing-copy-buffer"),
        });
    let command = CopyCommand::buffer_to_buffer(BufferHandle(703), BufferHandle(704), 4);
    assert_eq!(
        encode_copy_command(&mut encoder, &ResourceRegistry::new(), &command),
        Err(RenderGraphValidationError::MissingBuffer(BufferHandle(703)))
    );
}
