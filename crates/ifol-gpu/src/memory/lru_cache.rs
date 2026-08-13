use std::collections::{HashMap, HashSet};
use wgpu::TextureFormat;
use crate::memory::{SubmissionId, SubmissionTracker};
use crate::render::TextureHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureDimensionKey {
    D1,
    D2,
    D3,
}

/// Đầy đủ các thuộc tính ảnh hưởng khả năng tương thích của transient texture.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureDescriptorKey {
    pub width: u32,
    pub height: u32,
    pub depth_or_array_layers: u32,
    pub format: TextureFormat,
    pub usage: u32,
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub dimension: TextureDimensionKey,
}

impl TextureDescriptorKey {
    pub fn new(
        width: u32,
        height: u32,
        depth_or_array_layers: u32,
        format: TextureFormat,
        usage: wgpu::TextureUsages,
        mip_level_count: u32,
        sample_count: u32,
        dimension: TextureDimensionKey,
    ) -> Self {
        Self { width, height, depth_or_array_layers, format, usage: usage.bits(), mip_level_count, sample_count, dimension }
    }
}

#[derive(Debug, Clone, Copy)]
struct AvailableTexture {
    handle: TextureHandle,
    available_after: SubmissionId,
}

/// Free-list transient texture pool có bảo vệ theo submission completion.
pub struct TransientTexturePool {
    pools: HashMap<TextureDescriptorKey, Vec<AvailableTexture>>,
    known_handles: HashSet<TextureHandle>,
}

impl Default for TransientTexturePool {
    fn default() -> Self { Self::new() }
}

impl TransientTexturePool {
    pub fn new() -> Self {
        Self { pools: HashMap::new(), known_handles: HashSet::new() }
    }

    /// Chỉ lấy texture đã an toàn reuse tại completion hiện tại.
    pub fn acquire(&mut self, desc: &TextureDescriptorKey, tracker: &SubmissionTracker) -> Option<TextureHandle> {
        let completed = tracker.completed();
        let pool = self.pools.get_mut(desc)?;
        let index = pool.iter().rposition(|entry| entry.available_after <= completed)?;
        let entry = pool.swap_remove(index);
        self.known_handles.remove(&entry.handle);
        Some(entry.handle)
    }

    /// Đưa texture về pool sau submission cuối cùng có sử dụng nó.
    /// Release trùng handle bị từ chối để tránh cùng resource xuất hiện hai lần.
    pub fn release(&mut self, desc: TextureDescriptorKey, handle: TextureHandle, last_use: SubmissionId) -> bool {
        if !self.known_handles.insert(handle) { return false; }
        self.pools.entry(desc).or_default().push(AvailableTexture { handle, available_after: last_use });
        true
    }

    /// Trả các handle đã hoàn tất để host có thể giải phóng backing resource.
    pub fn pending_count(&self) -> usize {
        self.pools.values().map(Vec::len).sum()
    }

    pub fn drain_completed(&mut self, tracker: &SubmissionTracker) -> Vec<TextureHandle> {
        let completed = tracker.completed();
        let mut drained = Vec::new();
        for pool in self.pools.values_mut() {
            let mut kept = Vec::with_capacity(pool.len());
            for entry in pool.drain(..) {
                if entry.available_after <= completed { drained.push(entry.handle); }
                else { kept.push(entry); }
            }
            *pool = kept;
        }
        for handle in &drained { self.known_handles.remove(handle); }
        drained
    }
}

/// Tên cũ được giữ để source compatibility; semantics hiện tại là transient pool,
/// không còn là LRU và acquire/release đều yêu cầu submission contract.
pub type TextureCache = TransientTexturePool;

#[cfg(test)]
mod tests {
    use super::*;

    fn desc(format: TextureFormat) -> TextureDescriptorKey {
        TextureDescriptorKey::new(64, 32, 1, format, wgpu::TextureUsages::RENDER_ATTACHMENT, 1, 1, TextureDimensionKey::D2)
    }

    #[test]
    fn incompatible_descriptors_do_not_share_handles() {
        let mut pool = TransientTexturePool::new();
        let tracker = SubmissionTracker::new();
        assert!(pool.release(desc(TextureFormat::Rgba8Unorm), TextureHandle(1), SubmissionId(0)));
        assert_eq!(pool.acquire(&desc(TextureFormat::Bgra8Unorm), &tracker), None);
        assert_eq!(pool.acquire(&desc(TextureFormat::Rgba8Unorm), &tracker), Some(TextureHandle(1)));
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
}
