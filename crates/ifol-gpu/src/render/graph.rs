use crate::render::handle::{BindGroupHandle, MeshHandle, PipelineHandle, TextureHandle};

#[derive(Debug, Clone)]
pub enum DrawCommand {
    DrawMesh {
        mesh: MeshHandle,
        pipeline: PipelineHandle,
        bind_groups: Vec<BindGroupHandle>,
        instance_count: u32,
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

    /// Xuất đồ thị RenderGraph ra định dạng Mermaid (Markdown)
    pub fn export_mermaid(&self, filepath: &str) -> std::io::Result<()> {
        use std::io::Write;
        let mut out = String::new();
        out.push_str("```mermaid\n");
        out.push_str("graph TD\n");
        
        for (i, node) in self.nodes.iter().enumerate() {
            let node_id = format!("Node_{}", i);
            out.push_str(&format!("  {}[\"{}\"]\n", node_id, node.name));
            
            // Vẽ các Targets
            for (j, color) in node.target.color_attachments.iter().enumerate() {
                out.push_str(&format!("  {} --> {}_Color_{}[Color Target: {}]\n", node_id, node_id, j, color.0));
            }
            if let Some(depth) = &node.target.depth_attachment {
                out.push_str(&format!("  {} --> {}_Depth[Depth Target: {}]\n", node_id, node_id, depth.0));
            }
            
            // Vẽ Commands
            for (c, cmd) in node.commands.iter().enumerate() {
                match cmd {
                    DrawCommand::DrawMesh { mesh, pipeline, instance_count, .. } => {
                        out.push_str(&format!("  {} -.-> {}_Cmd_{}[Draw Mesh {} | Pipe {} | Inst {}]\n", 
                            node_id, node_id, c, mesh.0, pipeline.0, instance_count));
                    }
                }
            }
        }
        
        out.push_str("```\n");
        
        let mut file = std::fs::File::create(filepath)?;
        file.write_all(out.as_bytes())?;
        Ok(())
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
                mesh: MeshHandle(10),
                pipeline: PipelineHandle(1),
                bind_groups: vec![],
                instance_count: 1,
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
                instance_count: 1,
            });

        graph.add_node(shadow_node);
        graph.add_node(main_node);

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.nodes[0].name, "ShadowPass");
        assert_eq!(graph.nodes[1].commands.len(), 1);
    }
}
