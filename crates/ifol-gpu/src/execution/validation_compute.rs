use crate::graph::RenderNode;
use crate::resources::ResourceRegistry;

use super::validation::{
    bind_group_slot_index, validate_bind_group_offsets, validate_compute_pipeline_layout,
    validate_indirect_buffer, RenderGraphValidationError,
};

pub(crate) fn validate_compute_commands(
    registry: &ResourceRegistry,
    node: &RenderNode,
    max_bind_groups: u32,
) -> Result<(), RenderGraphValidationError> {
    for command in node.compute_commands() {
        if !registry.contains_compute_pipeline(&command.pipeline) {
            return Err(RenderGraphValidationError::MissingComputePipeline(
                command.pipeline,
            ));
        }
        for &(slot, bind_group, ref offsets) in &command.bind_groups {
            if bind_group_slot_index(slot, max_bind_groups).is_none() {
                return Err(RenderGraphValidationError::InvalidBindGroupSlot {
                    slot,
                    max_slots: max_bind_groups,
                });
            }
            if !registry.contains_bind_group(&bind_group) {
                return Err(RenderGraphValidationError::MissingBindGroup(bind_group));
            }
            validate_bind_group_offsets(registry, bind_group, offsets)?;
            validate_compute_pipeline_layout(registry, command.pipeline, slot, bind_group)?;
        }
        if let Some((buffer, offset)) = command.indirect {
            validate_indirect_buffer(registry, buffer, offset, 12)?;
        }
    }
    Ok(())
}
