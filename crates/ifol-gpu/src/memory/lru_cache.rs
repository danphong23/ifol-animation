use std::collections::HashMap;
use wgpu::TextureFormat;
use crate::render::TextureHandle;

/// Key dùng để định danh chính xác định dạng của Texture trong Pool
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureDescriptorKey {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    // (Tương lai có thể thêm usage, mip_level, sample_count...)
}

/// Cơ chế Exact-Match LRU đơn giản, tái sử dụng Texture để tránh cấp phát lại
pub struct TextureCache {
    pools: HashMap<TextureDescriptorKey, Vec<TextureHandle>>,
}

impl Default for TextureCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TextureCache {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
        }
    }

    /// Lấy ra một Texture tương thích từ Cache nếu có
    pub fn acquire(&mut self, desc: &TextureDescriptorKey) -> Option<TextureHandle> {
        self.pools.get_mut(desc).and_then(|pool| pool.pop())
    }

    /// Trả Texture về Cache để tái sử dụng ở frame sau
    pub fn release(&mut self, desc: TextureDescriptorKey, handle: TextureHandle) {
        self.pools.entry(desc).or_default().push(handle);
    }
}
