use crate::memory::{SubmissionId, SubmissionTracker};
use crate::resources::BufferHandle;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BufferDescriptorKey {
    pub size: u64,
    pub usage: u32,
}

impl BufferDescriptorKey {
    pub fn new(size: u64, usage: wgpu::BufferUsages) -> Self {
        Self {
            size,
            usage: usage.bits(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct AvailableBuffer {
    handle: BufferHandle,
    available_after: SubmissionId,
}

pub struct TransientBufferPool {
    pools: HashMap<BufferDescriptorKey, Vec<AvailableBuffer>>,
    known_handles: HashSet<BufferHandle>,
}

impl Default for TransientBufferPool {
    fn default() -> Self {
        Self::new()
    }
}

impl TransientBufferPool {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
            known_handles: HashSet::new(),
        }
    }

    pub fn acquire(
        &mut self,
        desc: &BufferDescriptorKey,
        tracker: &SubmissionTracker,
    ) -> Option<BufferHandle> {
        let completed = tracker.completed();
        let pool = self.pools.get_mut(desc)?;
        let index = pool
            .iter()
            .rposition(|entry| entry.available_after <= completed)?;
        let entry = pool.swap_remove(index);
        self.known_handles.remove(&entry.handle);
        Some(entry.handle)
    }

    pub fn release(
        &mut self,
        desc: BufferDescriptorKey,
        handle: BufferHandle,
        last_use: SubmissionId,
    ) -> bool {
        if !self.known_handles.insert(handle) {
            return false;
        }
        self.pools.entry(desc).or_default().push(AvailableBuffer {
            handle,
            available_after: last_use,
        });
        true
    }

    pub fn pending_count(&self) -> usize {
        self.pools.values().map(Vec::len).sum()
    }

    pub fn drain_completed(&mut self, tracker: &SubmissionTracker) -> Vec<BufferHandle> {
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
