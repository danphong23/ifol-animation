use super::descriptors::{ResourceDescriptorError, TextureResourceDescriptor};
use super::ResourceRegistry;
use crate::memory::{DeferredDestructionQueue, SubmissionId};
use crate::resources::handle::TextureHandle;

pub struct OwnedTextureResource {
    texture: wgpu::Texture,
    descriptor: TextureResourceDescriptor,
}

impl OwnedTextureResource {
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn descriptor(&self) -> TextureResourceDescriptor {
        self.descriptor
    }
}

impl ResourceRegistry {
    /// Lưu texture object thật cùng view compatibility. Đây là API cần cho
    /// copy/resolve; view-only registration chỉ lưu view và không đủ ownership.
    pub fn insert_owned_texture(
        &mut self,
        handle: TextureHandle,
        texture: wgpu::Texture,
        descriptor: TextureResourceDescriptor,
        max_dimension: u32,
    ) -> Result<Option<OwnedTextureResource>, ResourceDescriptorError> {
        descriptor.validate(max_dimension)?;
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let old = self.owned_textures.insert(
            handle,
            OwnedTextureResource {
                texture,
                descriptor,
            },
        );
        self.textures.insert(handle, (view, descriptor.format));
        self.texture_descriptors.insert(handle, descriptor);
        self.bump_texture_version(handle);
        Ok(old)
    }

    pub fn remove_owned_texture(&mut self, handle: &TextureHandle) -> Option<OwnedTextureResource> {
        let old = self.owned_textures.remove(handle);
        if old.is_some() {
            self.textures.remove(handle);
            self.texture_descriptors.remove(handle);
            self.bump_texture_version(*handle);
        }
        old
    }

    /// Tách texture khỏi registry nhưng giữ backing object tới sau submission
    /// cuối cùng dùng nó. Caller vẫn phải drain queue sau khi tracker báo hoàn tất.
    pub fn defer_owned_texture_destruction(
        &mut self,
        handle: &TextureHandle,
        last_use: SubmissionId,
        queue: &mut DeferredDestructionQueue<OwnedTextureResource>,
    ) -> bool {
        let Some(resource) = self.remove_owned_texture(handle) else {
            return false;
        };
        queue.defer(resource, last_use);
        true
    }
}
