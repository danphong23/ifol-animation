use crate::render::handle::{BindGroupHandle, MeshHandle, PipelineHandle, TextureHandle};

#[derive(Debug, Clone)]
pub enum DrawCommand {
    DrawMesh {
        mesh: MeshHandle,
        pipeline: PipelineHandle,
        bind_groups: Vec<BindGroupHandle>,
    },
}

#[derive(Debug, Clone)]
pub struct RenderTarget {
    pub color_attachments: Vec<TextureHandle>,
    pub depth_attachment: Option<TextureHandle>,
}

#[derive(Debug, Clone)]
pub struct RenderNode {
    pub name: String,
    pub target: RenderTarget,
    pub commands: Vec<DrawCommand>,
}

impl RenderNode {
    pub fn new(name: impl Into<String>, target: RenderTarget) -> Self {
        Self {
            name: name.into(),
            target,
            commands: Vec::new(),
        }
    }

    pub fn with_command(mut self, command: DrawCommand) -> Self {
        self.commands.push(command);
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct RenderGraph {
    pub nodes: Vec<RenderNode>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: RenderNode) {
        self.nodes.push(node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_graph_nesting() {
        let mut graph = RenderGraph::new();

        // Pass 1: Shadow Map
        let shadow_target = RenderTarget {
            color_attachments: vec![],
            depth_attachment: Some(TextureHandle(1)),
        };
        let shadow_node = RenderNode::new("ShadowPass", shadow_target)
            .with_command(DrawCommand::DrawMesh {
                mesh: MeshHandle(100),
                pipeline: PipelineHandle(10),
                bind_groups: vec![BindGroupHandle(1)],
            });
        
        // Pass 2: Main Forward Render
        let main_target = RenderTarget {
            color_attachments: vec![TextureHandle(2)],
            depth_attachment: Some(TextureHandle(3)), // Screen depth
        };
        let main_node = RenderNode::new("MainPass", main_target)
            .with_command(DrawCommand::DrawMesh {
                mesh: MeshHandle(100),
                pipeline: PipelineHandle(20),
                bind_groups: vec![BindGroupHandle(2)], // Giả sử bind_group này chứa ShadowMap texture
            });

        graph.add_node(shadow_node);
        graph.add_node(main_node);

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.nodes[0].name, "ShadowPass");
        assert_eq!(graph.nodes[1].commands.len(), 1);
    }
}
