use crate::extensions::ExtensionDispatchRegistry;
use crate::graph::{GraphResource, RenderGraph, RenderNode, RenderNodePool};
use crate::resources::ResourceRegistry;

use super::validation::{validate_graph, RenderGraphValidationError};
use super::validation_compute::validate_compute_commands;
use super::validation_copy::validate_copy_commands;
use super::validation_render::validate_render_commands;

pub(crate) fn validate_graph_nodes(
    registry: &ResourceRegistry,
    pool: &RenderNodePool,
    graph: &RenderGraph,
    max_bind_groups: u32,
    extension_dispatchers: &ExtensionDispatchRegistry,
) -> Result<(), RenderGraphValidationError> {
    for &node_id in &graph.node_ids {
        let node = pool
            .get(node_id)
            .ok_or(RenderGraphValidationError::MissingNode(node_id))?;
        if let RenderNode::Extension { extension, usages } = node {
            let Some(dispatcher) = extension_dispatchers.get(extension) else {
                return Err(RenderGraphValidationError::UnsupportedExtension(
                    extension.clone(),
                ));
            };
            dispatcher.validate(usages).map_err(|error| {
                RenderGraphValidationError::ExtensionValidation {
                    extension: extension.clone(),
                    error,
                }
            })?;
        }
        for usage in graph.resource_usages(&node_id) {
            match usage.resource {
                GraphResource::Buffer(handle) if !registry.contains_buffer(&handle) => {
                    return Err(RenderGraphValidationError::MissingUsageBuffer(handle));
                }
                GraphResource::Texture(handle) if !registry.contains_texture(&handle) => {
                    return Err(RenderGraphValidationError::MissingUsageTexture(handle));
                }
                _ => {}
            }
        }
        validate_render_commands(registry, node, max_bind_groups)?;
        validate_compute_commands(registry, node, max_bind_groups)?;
        validate_copy_commands(registry, node)?;
        if let RenderNode::SubGraph { graph: child, .. } = node {
            validate_graph(
                registry,
                pool,
                child,
                max_bind_groups,
                extension_dispatchers,
            )?;
        }
    }
    Ok(())
}
