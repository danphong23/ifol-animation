use super::*;
use crate::backend::GpuEngineBuilder;
use crate::memory::TextureDimensionKey;
use crate::resources::{ResourceRegistry, TextureResourceDescriptor};
use wgpu::TextureFormat;

#[test]
fn frame_context_seals_and_reopens_only_after_completion() {
    let texture_desc = TextureDescriptorKey::new(
        8,
        8,
        1,
        TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        1,
        1,
        TextureDimensionKey::D2,
    );
    let buffer_desc = BufferDescriptorKey::new(64, wgpu::BufferUsages::STORAGE);
    let mut texture_pool = TransientTexturePool::new();
    let mut buffer_pool = TransientBufferPool::new();
    let mut frame = FrameContext::new(3);
    frame
        .track_texture(texture_desc.clone(), TextureHandle(1))
        .unwrap();
    frame
        .track_buffer(buffer_desc.clone(), BufferHandle(2))
        .unwrap();
    let mut tracker = SubmissionTracker::new();
    let submission = tracker.begin();
    frame
        .seal(submission, &mut texture_pool, &mut buffer_pool)
        .unwrap();
    assert_eq!(frame.reset_after(&tracker, 4), Ok(false));
    tracker.mark_completed(submission);
    assert_eq!(frame.reset_after(&tracker, 4), Ok(true));
    assert_eq!(frame.frame_index(), 4);
    assert_eq!(
        texture_pool.acquire(&texture_desc, &tracker),
        Some(TextureHandle(1))
    );
    assert_eq!(
        buffer_pool.acquire(&buffer_desc, &tracker),
        Some(BufferHandle(2))
    );
}

#[test]
fn frame_context_routes_owned_texture_to_deferred_queue() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("frame_owned_texture_test"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
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
    registry
        .insert_owned_texture(TextureHandle(7), texture, descriptor, 1024)
        .unwrap();
    let mut frame = FrameContext::new(5);
    frame
        .defer_owned_texture(&mut registry, TextureHandle(7))
        .unwrap();
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
