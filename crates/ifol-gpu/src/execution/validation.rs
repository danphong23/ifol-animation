pub use super::validation_errors::RenderGraphValidationError;

use crate::extensions::ExtensionDispatchRegistry;
use crate::graph::{RenderGraph, RenderNodePool};
use crate::resources::ResourceRegistry;

#[cfg(test)]
pub(crate) use super::validation_copy::texture_supports_aspect;
#[cfg(test)]
pub(crate) use super::validation_copy::validate_copy_range;
pub(crate) use super::validation_copy::format_has_stencil;
pub(crate) use super::validation_indirect::validate_indirect_buffer;
pub(crate) use super::validation_layout::{
    bind_group_slot_index, validate_bind_group_offsets, validate_compute_pipeline_layout,
    validate_render_pipeline_layout,
};
use super::validation_node::validate_graph_nodes;
use super::validation_target::{validate_depth_stencil, validate_render_target};

pub(crate) fn validate_graph(
    registry: &ResourceRegistry,
    pool: &RenderNodePool,
    graph: &RenderGraph,
    max_bind_groups: u32,
    extension_dispatchers: &ExtensionDispatchRegistry,
) -> Result<(), RenderGraphValidationError> {
    graph.flatten(pool).map_err(|error| match error {
        crate::graph::GraphFlattenError::MissingNode(node) => {
            RenderGraphValidationError::MissingNode(node)
        }
        crate::graph::GraphFlattenError::Cycle(node) => {
            RenderGraphValidationError::DependencyCycle(node)
        }
        crate::graph::GraphFlattenError::DependencyNodeOutsideGraph(node) => {
            RenderGraphValidationError::DependencyOutsideGraph(node)
        }
    })?;

    let target_sample_count = validate_render_target(registry, &graph.target)?;
    validate_depth_stencil(registry, graph.depth_stencil, target_sample_count)?;
    validate_graph_nodes(
        registry,
        pool,
        graph,
        max_bind_groups,
        extension_dispatchers,
    )
}
