use super::validation::RenderGraphValidationError;
use crate::graph::{DrawAction, RenderGraph, RenderNode, RenderNodePool};
use crate::resources::handle::RenderNodeId;

pub(crate) fn execution_counts_for_graph(
    pool: &RenderNodePool,
    graph: &RenderGraph,
) -> Result<(usize, usize, usize, usize, usize, usize), RenderGraphValidationError> {
    let plan = graph.flatten(pool).map_err(map_graph_flatten_error)?;
    let mut draws = 0;
    let mut computes = 0;
    let mut copies = 0;
    let mut indirect = 0;
    let usages = declared_usage_count(pool, graph);
    for flat_node in &plan.nodes {
        let Some(node) = pool.get(flat_node.node_id) else {
            return Err(RenderGraphValidationError::MissingNode(flat_node.node_id));
        };
        draws += node.commands().len();
        computes += node.compute_commands().len();
        copies += node.copy_commands().len();
        indirect += node
            .commands()
            .iter()
            .filter(|command| {
                matches!(
                    command.action,
                    DrawAction::Indirect { .. } | DrawAction::IndexedIndirect { .. }
                )
            })
            .count();
        indirect += node
            .compute_commands()
            .iter()
            .filter(|command| command.indirect.is_some())
            .count();
    }
    Ok((plan.nodes.len(), draws, computes, copies, indirect, usages))
}

pub(crate) fn map_graph_flatten_error(
    error: crate::graph::GraphFlattenError,
) -> RenderGraphValidationError {
    match error {
        crate::graph::GraphFlattenError::MissingNode(node) => {
            RenderGraphValidationError::MissingNode(node)
        }
        crate::graph::GraphFlattenError::Cycle(node) => {
            RenderGraphValidationError::DependencyCycle(node)
        }
        crate::graph::GraphFlattenError::DependencyNodeOutsideGraph(node) => {
            RenderGraphValidationError::DependencyOutsideGraph(node)
        }
    }
}

pub(crate) fn declared_usage_count(pool: &RenderNodePool, graph: &RenderGraph) -> usize {
    graph.node_ids.iter().fold(0, |count, node_id| {
        let nested = match pool.get(*node_id) {
            Some(RenderNode::SubGraph { graph: child, .. }) => declared_usage_count(pool, child),
            _ => 0,
        };
        let extension_usage_count = pool
            .get(*node_id)
            .map_or(0, |node| node.extension_usages().len());
        count + graph.resource_usages(node_id).len() + extension_usage_count + nested
    })
}

pub(crate) fn owner_graph_for_flat_path<'a>(
    root: &'a RenderGraph,
    pool: &'a RenderNodePool,
    path: &[RenderNodeId],
) -> Result<&'a RenderGraph, RenderGraphValidationError> {
    let mut owner = root;
    for &ancestor_id in path.iter().take(path.len().saturating_sub(1)) {
        let Some(RenderNode::SubGraph { graph, .. }) = pool.get(ancestor_id) else {
            return Err(RenderGraphValidationError::MissingNode(ancestor_id));
        };
        owner = graph;
    }
    Ok(owner)
}

pub(crate) fn flat_plan_owner_path(node: &crate::graph::FlatRenderNode) -> Vec<RenderNodeId> {
    node.path[..node.path.len().saturating_sub(1)].to_vec()
}
