use crate::memory::{SubmissionId, SubmissionTracker};
use crate::resources::TextureHandle;
use std::collections::{HashMap, HashSet};
use wgpu::TextureFormat;

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
    #[expect(clippy::too_many_arguments, reason = "texture allocation key mirrors independent GPU descriptor fields")]
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
        Self {
            width,
            height,
            depth_or_array_layers,
            format,
            usage: usage.bits(),
            mip_level_count,
            sample_count,
            dimension,
        }
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
    fn default() -> Self {
        Self::new()
    }
}

impl TransientTexturePool {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
            known_handles: HashSet::new(),
        }
    }

    /// Chỉ lấy texture đã an toàn reuse tại completion hiện tại.
    pub fn acquire(
        &mut self,
        desc: &TextureDescriptorKey,
        tracker: &SubmissionTracker,
    ) -> Option<TextureHandle> {
        let completed = tracker.completed();
        let pool = self.pools.get_mut(desc)?;
        let index = pool
            .iter()
            .rposition(|entry| entry.available_after <= completed)?;
        let entry = pool.swap_remove(index);
        self.known_handles.remove(&entry.handle);
        Some(entry.handle)
    }

    /// Đưa texture về pool sau submission cuối cùng có sử dụng nó.
    /// Release trùng handle bị từ chối để tránh cùng resource xuất hiện hai lần.
    pub fn release(
        &mut self,
        desc: TextureDescriptorKey,
        handle: TextureHandle,
        last_use: SubmissionId,
    ) -> bool {
        if !self.known_handles.insert(handle) {
            return false;
        }
        self.pools.entry(desc).or_default().push(AvailableTexture {
            handle,
            available_after: last_use,
        });
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
                if entry.available_after <= completed {
                    drained.push(entry.handle);
                } else {
                    kept.push(entry);
                }
            }
            *pool = kept;
        }
        for handle in &drained {
            self.known_handles.remove(handle);
        }
        drained
    }
}
