use ifol_gpu::render::{RenderGraph, RenderTarget};

#[test]
fn test_render_graph_creation() {
    let graph = RenderGraph::new(RenderTarget::Screen);
    assert_eq!(graph.node_ids.len(), 0);
}
