use super::graph::RenderGraph;
use super::usage::buffer_subresource_range;
use super::{GraphResource, ResourceAccess, ResourceSubresource, ResourceUsage, TextureAspect};
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
}
