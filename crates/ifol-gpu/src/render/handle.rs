#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComputePipelineHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindGroupHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderNodeId(pub u64);

/// Trait chung cho các handle typed có thể được cấp phát theo generation.
pub trait GenerationalHandle: Copy {
    fn from_raw(raw: u64) -> Self;
    fn raw(self) -> u64;
}

macro_rules! impl_generational_handle {
    ($($handle:ty),+ $(,)?) => {
        $(
            impl GenerationalHandle for $handle {
                fn from_raw(raw: u64) -> Self { Self(raw) }
                fn raw(self) -> u64 { self.0 }
            }
        )+
    };
}

impl_generational_handle!(PipelineHandle, ComputePipelineHandle, TextureHandle, MeshHandle, BindGroupHandle);

/// Bộ cấp phát handle có generation, tách việc tái sử dụng slot khỏi resource store.
///
/// Generation bắt đầu từ 1. Khi generation đạt `u32::MAX`, slot bị retire thay vì
/// wrap về 0 để stale handle không thể sống lại do tràn số.
#[derive(Debug, Default)]
pub struct HandleAllocator {
    generations: Vec<u32>,
    retired: Vec<bool>,
    free: Vec<u32>,
}

impl HandleAllocator {
    pub fn new() -> Self { Self::default() }

    pub fn allocate<H: GenerationalHandle>(&mut self) -> H {
        let index = if let Some(index) = self.free.pop() {
            index
        } else {
            let index = self.generations.len() as u32;
            self.generations.push(1);
            self.retired.push(false);
            index
        };
        H::from_raw(Self::encode(index, self.generations[index as usize]))
    }

    pub fn release<H: GenerationalHandle>(&mut self, handle: H) -> bool {
        let Some((index, generation)) = self.decode(handle) else { return false };
        let slot = index as usize;
        if self.retired[slot] || self.generations[slot] != generation { return false }
        if generation == u32::MAX {
            self.retired[slot] = true;
        } else {
            self.generations[slot] += 1;
            self.free.push(index);
        }
        true
    }

    pub fn is_alive<H: GenerationalHandle>(&self, handle: H) -> bool {
        self.decode(handle)
            .map(|(index, generation)| {
                let slot = index as usize;
                !self.retired[slot] && self.generations[slot] == generation
            })
            .unwrap_or(false)
    }

    fn encode(index: u32, generation: u32) -> u64 {
        (u64::from(generation) << 32) | u64::from(index)
    }

    fn decode<H: GenerationalHandle>(&self, handle: H) -> Option<(u32, u32)> {
        let raw = handle.raw();
        let index = raw as u32;
        let generation = (raw >> 32) as u32;
        (generation != 0 && (index as usize) < self.generations.len()).then_some((index, generation))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_handle_is_rejected_after_reuse() {
        let mut allocator = HandleAllocator::new();
        let first = allocator.allocate::<TextureHandle>();
        assert!(allocator.is_alive(first));
        assert!(allocator.release(first));
        assert!(!allocator.is_alive(first));

        let second = allocator.allocate::<TextureHandle>();
        assert_ne!(first, second);
        assert!(allocator.is_alive(second));
        assert!(!allocator.release(first));
    }

    #[test]
    fn invalid_generation_is_rejected() {
        let mut allocator = HandleAllocator::new();
        let texture = allocator.allocate::<TextureHandle>();
        let invalid = TextureHandle(texture.raw() + (1_u64 << 32));

        assert!(!allocator.is_alive(invalid));
        assert!(!allocator.release(invalid));
    }
}
