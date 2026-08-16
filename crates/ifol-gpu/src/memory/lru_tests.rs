use super::*;

fn desc(format: TextureFormat) -> TextureDescriptorKey {
    TextureDescriptorKey::new(
        64,
        32,
        1,
        format,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        1,
        1,
        TextureDimensionKey::D2,
    )
}

fn buffer_desc(usage: wgpu::BufferUsages) -> BufferDescriptorKey {
    BufferDescriptorKey::new(256, usage)
}

#[test]
fn incompatible_descriptors_do_not_share_handles() {
    let mut pool = TransientTexturePool::new();
    let tracker = SubmissionTracker::new();
    assert!(pool.release(
        desc(TextureFormat::Rgba8Unorm),
        TextureHandle(1),
        SubmissionId(0)
    ));
    assert_eq!(
        pool.acquire(&desc(TextureFormat::Bgra8Unorm), &tracker),
        None
    );
    assert_eq!(
        pool.acquire(&desc(TextureFormat::Rgba8Unorm), &tracker),
        Some(TextureHandle(1))
    );
}

#[test]
fn in_flight_handle_cannot_be_reused_early() {
    let mut pool = TransientTexturePool::new();
    let desc = desc(TextureFormat::Rgba8Unorm);
    let mut tracker = SubmissionTracker::new();
    let submission = tracker.begin();
    assert!(pool.release(desc.clone(), TextureHandle(7), submission));
    assert_eq!(pool.acquire(&desc, &tracker), None);
    tracker.mark_completed(submission);
    assert_eq!(pool.acquire(&desc, &tracker), Some(TextureHandle(7)));
}

#[test]
fn duplicate_release_is_rejected_and_drain_returns_completed_entries() {
    let mut pool = TransientTexturePool::new();
    let desc = desc(TextureFormat::Rgba8Unorm);
    assert!(pool.release(desc.clone(), TextureHandle(3), SubmissionId(1)));
    assert!(!pool.release(desc, TextureHandle(3), SubmissionId(2)));
    assert_eq!(pool.pending_count(), 1);
    let mut tracker = SubmissionTracker::new();
    tracker.mark_completed(SubmissionId(1));
    assert_eq!(pool.drain_completed(&tracker), vec![TextureHandle(3)]);
    assert_eq!(pool.pending_count(), 0);
}

#[test]
fn transient_buffer_pool_respects_descriptor_and_submission() {
    let mut pool = TransientBufferPool::new();
    let desc = buffer_desc(wgpu::BufferUsages::STORAGE);
    let mut tracker = SubmissionTracker::new();
    let submission = tracker.begin();
    assert!(pool.release(desc.clone(), BufferHandle(8), submission));
    assert_eq!(pool.acquire(&desc, &tracker), None);
    assert_eq!(
        pool.acquire(&buffer_desc(wgpu::BufferUsages::UNIFORM), &tracker),
        None
    );
    tracker.mark_completed(submission);
    assert_eq!(pool.acquire(&desc, &tracker), Some(BufferHandle(8)));
}

#[test]
fn transient_buffer_pool_rejects_duplicate_release_and_drains() {
    let mut pool = TransientBufferPool::new();
    let desc = buffer_desc(wgpu::BufferUsages::INDIRECT);
    assert!(pool.release(desc.clone(), BufferHandle(9), SubmissionId(2)));
    assert!(!pool.release(desc, BufferHandle(9), SubmissionId(3)));
    let mut tracker = SubmissionTracker::new();
    tracker.mark_completed(SubmissionId(2));
    assert_eq!(pool.drain_completed(&tracker), vec![BufferHandle(9)]);
}
