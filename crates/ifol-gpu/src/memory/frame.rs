use std::collections::HashSet;
use thiserror::Error;
use crate::memory::{BufferDescriptorKey, DeferredDestructionQueue, SubmissionId, SubmissionTracker, TextureDescriptorKey, TransientBufferPool, TransientTexturePool};
use crate::render::{BufferHandle, OwnedTextureResource, ResourceRegistry, TextureHandle};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FrameContextError {
    #[error("frame context has already been sealed")]
    AlreadySealed,
    #[error("frame context has not been sealed")]
    NotSealed,
    #[error("resource handle is already tracked by this frame")]
    DuplicateResource,
    #[error("transient resource release was rejected by its pool")]
    ReleaseRejected,
    #[error("owned texture is missing from the registry")]
    MissingOwnedTexture,
    #[error("frame contains owned textures that require seal_with_deferred_textures")]
    DeferredDestructionQueueRequired,
}

pub struct FrameContext {
    frame_index: u64,
    submission: Option<SubmissionId>,
    textures: Vec<(TextureDescriptorKey, TextureHandle)>,
    buffers: Vec<(BufferDescriptorKey, BufferHandle)>,
    texture_handles: HashSet<TextureHandle>,
    buffer_handles: HashSet<BufferHandle>,
    pending_owned_textures: Vec<OwnedTextureResource>,
    pending_owned_texture_handles: HashSet<TextureHandle>,
}

impl FrameContext {
    pub fn new(frame_index: u64) -> Self {
        Self {
            frame_index,
            submission: None,
            textures: Vec::new(),
            buffers: Vec::new(),
            texture_handles: HashSet::new(),
            buffer_handles: HashSet::new(),
            pending_owned_textures: Vec::new(),
            pending_owned_texture_handles: HashSet::new(),
        }
    }

    pub fn frame_index(&self) -> u64 { self.frame_index }
    pub fn submission(&self) -> Option<SubmissionId> { self.submission }

    pub fn track_texture(&mut self, desc: TextureDescriptorKey, handle: TextureHandle) -> Result<(), FrameContextError> {
        if self.submission.is_some() || !self.texture_handles.insert(handle) {
            return Err(if self.submission.is_some() { FrameContextError::AlreadySealed } else { FrameContextError::DuplicateResource });
        }
        self.textures.push((desc, handle));
        Ok(())
    }

    pub fn track_buffer(&mut self, desc: BufferDescriptorKey, handle: BufferHandle) -> Result<(), FrameContextError> {
        if self.submission.is_some() || !self.buffer_handles.insert(handle) {
            return Err(if self.submission.is_some() { FrameContextError::AlreadySealed } else { FrameContextError::DuplicateResource });
        }
        self.buffers.push((desc, handle));
        Ok(())
    }

    /// Chuyển ownership texture ra khỏi registry, nhưng giữ object trong frame
    /// cho tới lúc seal để gắn nó với submission cuối cùng.
    pub fn defer_owned_texture(
        &mut self,
        registry: &mut ResourceRegistry,
        handle: TextureHandle,
    ) -> Result<(), FrameContextError> {
        if self.submission.is_some() {
            return Err(FrameContextError::AlreadySealed);
        }
        if !self.pending_owned_texture_handles.insert(handle) {
            return Err(FrameContextError::DuplicateResource);
        }
        let Some(resource) = registry.remove_owned_texture(&handle) else {
            self.pending_owned_texture_handles.remove(&handle);
            return Err(FrameContextError::MissingOwnedTexture);
        };
        self.pending_owned_textures.push(resource);
        Ok(())
    }

    pub fn seal(
        &mut self,
        submission: SubmissionId,
        textures: &mut TransientTexturePool,
        buffers: &mut TransientBufferPool,
    ) -> Result<(), FrameContextError> {
        if !self.pending_owned_textures.is_empty() {
            return Err(FrameContextError::DeferredDestructionQueueRequired);
        }
        self.seal_transients(submission, textures, buffers)
    }

    /// Seal frame và đưa owned textures đã tách khỏi registry vào queue có
    /// completion gate của submission hiện tại.
    pub fn seal_with_deferred_textures(
        &mut self,
        submission: SubmissionId,
        textures: &mut TransientTexturePool,
        buffers: &mut TransientBufferPool,
        deferred_textures: &mut DeferredDestructionQueue<OwnedTextureResource>,
    ) -> Result<(), FrameContextError> {
        self.seal_transients(submission, textures, buffers)?;
        for resource in self.pending_owned_textures.drain(..) {
            deferred_textures.defer(resource, submission);
        }
        self.pending_owned_texture_handles.clear();
        Ok(())
    }

    fn seal_transients(
        &mut self,
        submission: SubmissionId,
        textures: &mut TransientTexturePool,
        buffers: &mut TransientBufferPool,
    ) -> Result<(), FrameContextError> {
        if self.submission.is_some() { return Err(FrameContextError::AlreadySealed); }
        for (desc, handle) in &self.textures {
            if !textures.release(desc.clone(), *handle, submission) { return Err(FrameContextError::ReleaseRejected); }
        }
        for (desc, handle) in &self.buffers {
            if !buffers.release(desc.clone(), *handle, submission) { return Err(FrameContextError::ReleaseRejected); }
        }
        self.submission = Some(submission);
        Ok(())
    }

    pub fn reset_after(&mut self, tracker: &SubmissionTracker, next_frame_index: u64) -> Result<bool, FrameContextError> {
        let Some(submission) = self.submission else { return Err(FrameContextError::NotSealed); };
        if !tracker.can_reuse_after(submission) { return Ok(false); }
        self.frame_index = next_frame_index;
        self.submission = None;
        self.textures.clear();
        self.buffers.clear();
        self.texture_handles.clear();
        self.buffer_handles.clear();
        self.pending_owned_texture_handles.clear();
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::GpuEngineBuilder;
    use crate::memory::TextureDimensionKey;
    use crate::render::{ResourceRegistry, TextureResourceDescriptor};
    use wgpu::TextureFormat;

    #[test]
    fn frame_context_seals_and_reopens_only_after_completion() {
        let texture_desc = TextureDescriptorKey::new(8, 8, 1, TextureFormat::Rgba8Unorm, wgpu::TextureUsages::RENDER_ATTACHMENT, 1, 1, TextureDimensionKey::D2);
        let buffer_desc = BufferDescriptorKey::new(64, wgpu::BufferUsages::STORAGE);
        let mut texture_pool = TransientTexturePool::new();
        let mut buffer_pool = TransientBufferPool::new();
        let mut frame = FrameContext::new(3);
        frame.track_texture(texture_desc.clone(), TextureHandle(1)).unwrap();
        frame.track_buffer(buffer_desc.clone(), BufferHandle(2)).unwrap();
        let mut tracker = SubmissionTracker::new();
        let submission = tracker.begin();
        frame.seal(submission, &mut texture_pool, &mut buffer_pool).unwrap();
        assert_eq!(frame.reset_after(&tracker, 4), Ok(false));
        tracker.mark_completed(submission);
        assert_eq!(frame.reset_after(&tracker, 4), Ok(true));
        assert_eq!(frame.frame_index(), 4);
        assert_eq!(texture_pool.acquire(&texture_desc, &tracker), Some(TextureHandle(1)));
        assert_eq!(buffer_pool.acquire(&buffer_desc, &tracker), Some(BufferHandle(2)));
    }

    #[test]
    fn frame_context_routes_owned_texture_to_deferred_queue() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("frame_owned_texture_test"),
            size: wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let descriptor = TextureResourceDescriptor {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            mip_level_count: 1,
            sample_count: 1,
        };
        let mut registry = ResourceRegistry::new();
        registry.insert_owned_texture(TextureHandle(7), texture, descriptor, 1024).unwrap();
        let mut frame = FrameContext::new(5);
        frame.defer_owned_texture(&mut registry, TextureHandle(7)).unwrap();
        let mut textures = TransientTexturePool::new();
        let mut buffers = TransientBufferPool::new();
        let mut deferred = DeferredDestructionQueue::new();
        let mut tracker = SubmissionTracker::new();
        let submission = tracker.begin();
        assert_eq!(
            frame.seal(submission, &mut textures, &mut buffers),
            Err(FrameContextError::DeferredDestructionQueueRequired)
        );
        frame
            .seal_with_deferred_textures(submission, &mut textures, &mut buffers, &mut deferred)
            .unwrap();
        assert_eq!(deferred.pending_count(), 1);
        assert!(deferred.drain_completed(&tracker).is_empty());
        tracker.mark_completed(submission);
        assert_eq!(deferred.drain_completed(&tracker).len(), 1);
        assert!(frame.reset_after(&tracker, 6).unwrap());
    }

    #[test]
    fn frame_context_rejects_missing_owned_texture_without_reserving_handle() {
        let mut registry = ResourceRegistry::new();
        let mut frame = FrameContext::new(0);
        assert_eq!(
            frame.defer_owned_texture(&mut registry, TextureHandle(404)),
            Err(FrameContextError::MissingOwnedTexture)
        );
        assert_eq!(
            frame.defer_owned_texture(&mut registry, TextureHandle(404)),
            Err(FrameContextError::MissingOwnedTexture)
        );
    }
}
