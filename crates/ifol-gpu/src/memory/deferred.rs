use crate::memory::{SubmissionId, SubmissionTracker};

struct DeferredResource<T> {
    resource: T,
    available_after: SubmissionId,
}

pub struct DeferredDestructionQueue<T> {
    pending: Vec<DeferredResource<T>>,
}

impl<T> Default for DeferredDestructionQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> DeferredDestructionQueue<T> {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    pub fn defer(&mut self, resource: T, last_use: SubmissionId) {
        self.pending.push(DeferredResource {
            resource,
            available_after: last_use,
        });
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn drain_completed(&mut self, tracker: &SubmissionTracker) -> Vec<T> {
        let completed = tracker.completed();
        let mut ready = Vec::new();
        let mut pending = Vec::with_capacity(self.pending.len());
        for entry in self.pending.drain(..) {
            if entry.available_after <= completed {
                ready.push(entry.resource);
            } else {
                pending.push(entry);
            }
        }
        self.pending = pending;
        ready
    }
}

#[cfg(test)]
#[path = "deferred_tests.rs"]
mod tests;
