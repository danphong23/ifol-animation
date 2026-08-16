use crate::resources::ResourceRegistry;

use super::validation::{bind_group_slot_index, RenderGraphValidationError};

pub(crate) fn encode_compute_commands(
    encoder: &mut wgpu::CommandEncoder,
    registry: &ResourceRegistry,
    commands: &[crate::graph::ComputeCommand],
    max_bind_groups: u32,
) -> Result<(), RenderGraphValidationError> {
    if commands.is_empty() {
        return Ok(());
    }
    let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("RenderGraphComputePass"),
        timestamp_writes: None,
    });
    let mut current_pipeline = None;
    let mut current_bind_groups = vec![None; max_bind_groups as usize];
    for command in commands {
        if current_pipeline != Some(command.pipeline) {
            let Some(pipeline) = registry.compute_pipeline(&command.pipeline) else {
                return Err(RenderGraphValidationError::MissingComputePipeline(
                    command.pipeline,
                ));
            };
            compute_pass.set_pipeline(pipeline);
            current_pipeline = Some(command.pipeline);
        }
        for &(slot, bind_group, ref offsets) in &command.bind_groups {
            let Some(slot_index) = bind_group_slot_index(slot, max_bind_groups) else {
                return Err(RenderGraphValidationError::InvalidBindGroupSlot {
                    slot,
                    max_slots: max_bind_groups,
                });
            };
            if current_bind_groups[slot_index] != Some(bind_group) || !offsets.is_empty() {
                let Some(group) = registry.bind_group(&bind_group) else {
                    return Err(RenderGraphValidationError::MissingBindGroup(bind_group));
                };
                compute_pass.set_bind_group(slot, group, offsets);
                current_bind_groups[slot_index] = Some(bind_group);
            }
        }
        if let Some((buffer, offset)) = command.indirect {
            let Some(indirect) = registry.buffer(&buffer) else {
                return Err(RenderGraphValidationError::MissingIndirectBuffer(buffer));
            };
            compute_pass.dispatch_workgroups_indirect(indirect, offset);
        } else {
            compute_pass.dispatch_workgroups(
                command.workgroups[0],
                command.workgroups[1],
                command.workgroups[2],
            );
        }
    }
    Ok(())
}
