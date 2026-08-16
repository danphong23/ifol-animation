use crate::graph::{DrawAction, RenderNode};
use crate::resources::ResourceRegistry;

use super::validation::{
    bind_group_slot_index, validate_bind_group_offsets, validate_indirect_buffer,
    validate_render_pipeline_layout, RenderGraphValidationError,
};

pub(crate) fn validate_render_commands(
    registry: &ResourceRegistry,
    node: &RenderNode,
    max_bind_groups: u32,
) -> Result<(), RenderGraphValidationError> {
    for command in node.commands() {
        if !registry.contains_pipeline(&command.pipeline) {
            return Err(RenderGraphValidationError::MissingPipeline(
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
            validate_render_pipeline_layout(registry, command.pipeline, slot, bind_group)?;
        }
        if let DrawAction::Indexed { mesh, .. } = command.action {
            if !registry.contains_mesh(&mesh) {
                return Err(RenderGraphValidationError::MissingMesh(mesh));
            }
        }
        match command.action {
            DrawAction::Indirect { buffer, offset } => {
                validate_indirect_buffer(registry, buffer, offset, 16)?;
            }
            DrawAction::IndexedIndirect {
                mesh,
                buffer,
                offset,
            } => {
                let Some((_, Some(_), _)) = registry.mesh(&mesh) else {
                    if !registry.contains_mesh(&mesh) {
                        return Err(RenderGraphValidationError::MissingMesh(mesh));
                    }
                    return Err(RenderGraphValidationError::MissingIndexBuffer(mesh));
                };
                validate_indirect_buffer(registry, buffer, offset, 20)?;
            }
            _ => {}
        }
    }
    Ok(())
}
