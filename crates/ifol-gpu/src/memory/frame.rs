use std::collections::HashSet;
use thiserror::Error;
use crate::memory::{BufferDescriptorKey, SubmissionId, SubmissionTracker, TextureDescriptorKey, TransientBufferPool, TransientTexturePool};
use crate::render::{BufferHandle, TextureHandle};

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
}

pub struct FrameContext {
    frame_index: u64,
    submission: Option<SubmissionId>,
    textures: Vec<(TextureDescriptorKey, TextureHandle)>,
    buffers: Vec<(BufferDescriptorKey, BufferHandle)>,
    texture_handles: HashSet<TextureHandle>,
    buffer_handles: HashSet<BufferHandle>,
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

    pub fn seal(
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
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::TextureDimensionKey;
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
}
