use crate::resources::handle::{BindGroupHandle, ComputePipelineHandle, PipelineHandle};
use crate::resources::ResourceRegistry;

use super::validation::RenderGraphValidationError;

pub(crate) fn bind_group_slot_index(slot: u32, max_slots: u32) -> Option<usize> {
    (slot < max_slots).then_some(slot as usize)
}

pub(crate) fn validate_bind_group_offsets(
    registry: &ResourceRegistry,
    handle: BindGroupHandle,
    offsets: &[u32],
) -> Result<(), RenderGraphValidationError> {
    let Some(descriptor) = registry.bind_group_descriptor(&handle) else {
        return Ok(());
    };
    if offsets.len() as u32 != descriptor.dynamic_offset_count {
        return Err(RenderGraphValidationError::InvalidDynamicOffsetCount {
            handle,
            expected: descriptor.dynamic_offset_count,
            actual: offsets.len() as u32,
        });
    }
    for &offset in offsets {
        if offset % descriptor.dynamic_offset_alignment != 0 {
            return Err(RenderGraphValidationError::InvalidDynamicOffsetAlignment {
                handle,
                offset,
                alignment: descriptor.dynamic_offset_alignment,
            });
        }
    }
    Ok(())
}

pub(crate) fn validate_render_pipeline_layout(
    registry: &ResourceRegistry,
    pipeline: PipelineHandle,
    slot: u32,
    bind_group: BindGroupHandle,
) -> Result<(), RenderGraphValidationError> {
    let Some(descriptor) = registry.pipeline_layout_descriptor(&pipeline) else {
        return Ok(());
    };
    let expected = descriptor
        .bind_group_layout_signatures
        .get(slot as usize)
        .copied()
        .flatten();
    let actual = registry
        .bind_group_descriptor(&bind_group)
        .map(|descriptor| descriptor.layout_signature);
    if expected.is_some() && actual.is_none() {
        return Err(RenderGraphValidationError::MissingPipelineLayoutMetadata {
            pipeline,
            bind_group,
            slot,
        });
    }
    if expected != actual {
        return Err(RenderGraphValidationError::PipelineLayoutMismatch {
            pipeline,
            slot,
            expected,
            actual,
        });
    }
    Ok(())
}

pub(crate) fn validate_compute_pipeline_layout(
    registry: &ResourceRegistry,
    pipeline: ComputePipelineHandle,
    slot: u32,
    bind_group: BindGroupHandle,
) -> Result<(), RenderGraphValidationError> {
    let Some(descriptor) = registry.compute_pipeline_layout_descriptor(&pipeline) else {
        return Ok(());
    };
    let expected = descriptor
        .bind_group_layout_signatures
        .get(slot as usize)
        .copied()
        .flatten();
    let actual = registry
        .bind_group_descriptor(&bind_group)
        .map(|descriptor| descriptor.layout_signature);
    if expected.is_some() && actual.is_none() {
        return Err(
            RenderGraphValidationError::MissingComputePipelineLayoutMetadata {
                pipeline,
                bind_group,
                slot,
            },
        );
    }
    if expected != actual {
        return Err(RenderGraphValidationError::ComputePipelineLayoutMismatch {
            pipeline,
            slot,
            expected,
            actual,
        });
    }
    Ok(())
}
