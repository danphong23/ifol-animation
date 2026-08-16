use crate::resources::handle::RenderNodeId;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatRenderNode {
    pub node_id: RenderNodeId,
    /// Chuỗi node từ root tới node này, dùng cho diagnostics/profiling.
    pub path: Vec<RenderNodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FlatRenderPlan {
    pub nodes: Vec<FlatRenderNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphDependency {
    pub before: RenderNodeId,
    pub after: RenderNodeId,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GraphFlattenError {
    #[error("render node {0:?} does not exist in the node pool")]
    MissingNode(RenderNodeId),
    #[error("cycle detected while flattening render graph at node {0:?}")]
    Cycle(RenderNodeId),
    #[error("dependency references node {0:?} outside the graph")]
    DependencyNodeOutsideGraph(RenderNodeId),
}

impl FlatRenderPlan {
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}
