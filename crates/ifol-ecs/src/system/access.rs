use crate::registry::ComponentId;
use std::collections::HashSet;

/// Declares component types accessed (read or write) by a system.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccessDescriptor {
    pub reads: HashSet<ComponentId>,
    pub writes: HashSet<ComponentId>,
}

impl AccessDescriptor {
    /// Creates a new empty access descriptor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a read dependency on the given component.
    pub fn add_read(&mut self, id: ComponentId) {
        self.reads.insert(id);
    }

    /// Adds a write dependency on the given component.
    pub fn add_write(&mut self, id: ComponentId) {
        self.writes.insert(id);
    }

    #[inline]
    pub(crate) fn allows_read(&self, id: ComponentId) -> bool {
        self.reads.contains(&id) || self.writes.contains(&id)
    }

    #[inline]
    pub(crate) fn allows_write(&self, id: ComponentId) -> bool {
        self.writes.contains(&id)
    }

    /// Validates that there are no internal conflicting read/write accesses.
    pub fn validate(&self) -> Result<(), &'static str> {
        for write_id in &self.writes {
            if self.reads.contains(write_id) {
                return Err(
                    "Component cannot be declared as both read and write in the same access descriptor",
                );
            }
        }
        Ok(())
    }
}
