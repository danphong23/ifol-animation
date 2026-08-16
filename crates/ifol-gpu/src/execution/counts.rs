use super::orchestration::map_graph_flatten_error;
use super::validation::RenderGraphValidationError;
use crate::graph::{DrawAction, RenderGraph, RenderNode, RenderNodePool};

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
