#[path = "texture_pool.rs"]
mod texture_pool;
#[path = "buffer_pool.rs"]
mod buffer_pool;

pub use buffer_pool::{BufferDescriptorKey, TransientBufferPool};
pub use texture_pool::{TextureDescriptorKey, TextureDimensionKey, TransientTexturePool};

#[cfg(test)]
#[path = "lru_tests.rs"]
mod tests;
