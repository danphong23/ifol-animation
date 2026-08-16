use super::graph::RenderGraph;
use super::usage::{
    buffer_subresource_range, texture_aspect_subresource_range, texture_subresource_range,
};
use super::{
    CopyCommand, DrawAction, GraphResource, RenderNodePool, RenderTarget, ResourceAccess,
    ResourceSubresource, ResourceUsage, TextureAspect,
};
use crate::resources::handle::{BufferHandle, RenderNodeId, TextureHandle};

impl RenderGraph {
    /// Khai báo resource mà node đọc/ghi. Đây là metadata cho hazard compiler;
    /// command encoder hiện tại vẫn giữ behavior cũ nếu graph không khai báo.
    pub fn declare_resource_usage(
        &mut self,
        node: RenderNodeId,
        resource: GraphResource,
        access: ResourceAccess,
    ) {
        self.resource_usages
            .entry(node)
            .or_default()
            .push(ResourceUsage {
                resource,
                access,
                subresource: ResourceSubresource::Whole,
            });
    }

    pub fn declare_texture_subresource_usage(
        &mut self,
        node: RenderNodeId,
        texture: TextureHandle,
        mip_level: u32,
        array_layer: u32,
        access: ResourceAccess,
    ) {
        self.resource_usages
            .entry(node)
            .or_default()
            .push(ResourceUsage {
                resource: GraphResource::Texture(texture),
                access,
                subresource: ResourceSubresource::Texture {
                    mip_level,
                    array_layer,
                },
            });
    }

    pub fn declare_texture_subresource_range_usage(
        &mut self,
        node: RenderNodeId,
        texture: TextureHandle,
        mip_start: u32,
        mip_end: u32,
        layer_start: u32,
        layer_end: u32,
        access: ResourceAccess,
    ) {
        self.resource_usages
            .entry(node)
            .or_default()
            .push(ResourceUsage {
                resource: GraphResource::Texture(texture),
                access,
                subresource: ResourceSubresource::TextureRange {
                    mip_start,
                    mip_end,
                    layer_start,
                    layer_end,
                },
            });
    }

    pub fn declare_texture_aspect_usage(
        &mut self,
        node: RenderNodeId,
        texture: TextureHandle,
        mip_level: u32,
        array_layer: u32,
        aspect: TextureAspect,
        access: ResourceAccess,
    ) {
        self.resource_usages
            .entry(node)
            .or_default()
            .push(ResourceUsage {
                resource: GraphResource::Texture(texture),
                access,
                subresource: ResourceSubresource::TextureAspect {
                    mip_level,
                    array_layer,
                    aspect,
                },
            });
    }

    pub fn declare_texture_aspect_range_usage(
        &mut self,
        node: RenderNodeId,
        texture: TextureHandle,
        mip_start: u32,
        mip_end: u32,
        layer_start: u32,
        layer_end: u32,
        aspect: TextureAspect,
        access: ResourceAccess,
    ) {
        self.resource_usages
            .entry(node)
            .or_default()
            .push(ResourceUsage {
                resource: GraphResource::Texture(texture),
                access,
                subresource: ResourceSubresource::TextureAspectRange {
                    mip_start,
                    mip_end,
                    layer_start,
                    layer_end,
                    aspect,
                },
            });
    }

    pub fn declare_buffer_range_usage(
        &mut self,
        node: RenderNodeId,
        buffer: BufferHandle,
        offset: u64,
        size: u64,
        access: ResourceAccess,
    ) {
        self.resource_usages
            .entry(node)
            .or_default()
            .push(ResourceUsage {
                resource: GraphResource::Buffer(buffer),
                access,
                subresource: buffer_subresource_range(offset, size),
            });
    }

    pub fn resource_usages(&self, node: &RenderNodeId) -> &[ResourceUsage] {
        self.resource_usages
            .get(node)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn effective_resource_usages(
        &self,
        node_id: RenderNodeId,
        pool: &RenderNodePool,
    ) -> Vec<ResourceUsage> {
        let mut usages = self.resource_usages(&node_id).to_vec();
        if let Some(node) = pool.get(node_id) {
            usages.extend_from_slice(node.extension_usages());
            for command in node.copy_commands() {
                match command {
                    CopyCommand::BufferToBuffer {
                        source,
                        destination,
                        source_offset,
                        destination_offset,
                        size,
                    } => {
                        usages.push(ResourceUsage {
                            resource: GraphResource::Buffer(*source),
                            access: ResourceAccess::Read,
                            subresource: buffer_subresource_range(*source_offset, *size),
                        });
                        usages.push(ResourceUsage {
                            resource: GraphResource::Buffer(*destination),
                            access: ResourceAccess::Write,
                            subresource: buffer_subresource_range(*destination_offset, *size),
                        });
                    }
                    CopyCommand::TextureToTexture {
                        source,
                        destination,
                        source_mip_level,
                        destination_mip_level,
                        source_origin,
                        destination_origin,
                        extent,
                    } => {
                        let source_subresource =
                            texture_subresource_range(*source_mip_level, *source_origin, *extent);
                        let destination_subresource = texture_subresource_range(
                            *destination_mip_level,
                            *destination_origin,
                            *extent,
                        );
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(*source),
                            access: ResourceAccess::Read,
                            subresource: source_subresource,
                        });
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(*destination),
                            access: ResourceAccess::Write,
                            subresource: destination_subresource,
                        });
                    }
                    CopyCommand::TextureToTextureAspect {
                        source,
                        destination,
                        source_mip_level,
                        destination_mip_level,
                        source_origin,
                        destination_origin,
                        extent,
                        aspect,
                    } => {
                        let source_subresource = texture_aspect_subresource_range(
                            *source_mip_level,
                            *source_origin,
                            *extent,
                            *aspect,
                        );
                        let destination_subresource = texture_aspect_subresource_range(
                            *destination_mip_level,
                            *destination_origin,
                            *extent,
                            *aspect,
                        );
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(*source),
                            access: ResourceAccess::Read,
                            subresource: source_subresource,
                        });
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(*destination),
                            access: ResourceAccess::Write,
                            subresource: destination_subresource,
                        });
                    }
                }
            }
            for command in node.commands() {
                let indirect = match command.action {
                    DrawAction::Indirect { buffer, offset } => Some((buffer, offset, 16)),
                    DrawAction::IndexedIndirect { buffer, offset, .. } => {
                        Some((buffer, offset, 20))
                    }
                    _ => None,
                };
                if let Some((buffer, offset, size)) = indirect {
                    usages.push(ResourceUsage {
                        resource: GraphResource::Buffer(buffer),
                        access: ResourceAccess::Read,
                        subresource: buffer_subresource_range(offset, size),
                    });
                }
            }
            for command in node.compute_commands() {
                if let Some((buffer, offset)) = command.indirect {
                    usages.push(ResourceUsage {
                        resource: GraphResource::Buffer(buffer),
                        access: ResourceAccess::Read,
                        subresource: buffer_subresource_range(offset, 12),
                    });
                }
            }
            if !node.commands().is_empty() {
                match self.target {
                    RenderTarget::Offscreen { color, .. } => {
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(color),
                            access: ResourceAccess::Write,
                            subresource: ResourceSubresource::Whole,
                        });
                    }
                    RenderTarget::OffscreenMsaa { color, resolve, .. } => {
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(color),
                            access: ResourceAccess::Write,
                            subresource: ResourceSubresource::Whole,
                        });
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(resolve),
                            access: ResourceAccess::Write,
                            subresource: ResourceSubresource::Whole,
                        });
                    }
                    RenderTarget::Screen => {}
                }
                if let Some(depth) = self.depth_stencil {
                    usages.push(ResourceUsage {
                        resource: GraphResource::Texture(depth),
                        access: ResourceAccess::Write,
                        subresource: ResourceSubresource::Whole,
                    });
                }
            }
        }
        usages
    }
}
