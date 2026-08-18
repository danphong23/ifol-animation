use crate::entity::EntityId;
use crate::storage::component::Component;

/// A contiguous, cache-friendly storage for components of type `T`.
///
/// Uses a sparse vector indexing a packed dense vector, providing:
/// - $O(1)$ component insertion, removal, and lookup.
/// - $O(N)$ linear iteration across contiguous memory without holes.
/// - Integrated tick-based change tracking per entity.
#[derive(Debug, Clone)]
pub struct SparseSet<T> {
    /// Sparse array: maps entity index to (dense_index + 1). 0 indicates absence.
    sparse: Vec<usize>,
    /// Packed dense array of entity IDs.
    dense_entities: Vec<EntityId>,
    /// Packed dense array of component data.
    dense_data: Vec<T>,
    /// Packed dense array of last-modified tick timestamps.
    dense_ticks: Vec<u64>,
}

impl<T: Component> SparseSet<T> {
    /// Creates a new empty `SparseSet`.
    pub fn new() -> Self {
        Self {
            sparse: Vec::new(),
            dense_entities: Vec::new(),
            dense_data: Vec::new(),
            dense_ticks: Vec::new(),
        }
    }

    /// Inserts or replaces a component for the given entity at the specified tick.
    ///
    /// Returns the previous component value if one existed.
    pub fn insert(&mut self, entity: EntityId, value: T, tick: u64) -> Option<T> {
        let entity_idx = entity.index() as usize;

        // Ensure sparse vector is large enough
        if entity_idx >= self.sparse.len() {
            self.sparse.resize(entity_idx + 1, 0);
        }

        let dense_slot = self.sparse[entity_idx];
        if dense_slot > 0 {
            // Already present: replace in-place
            let dense_idx = dense_slot - 1;
            self.dense_entities[dense_idx] = entity;
            self.dense_ticks[dense_idx] = tick;
            let old = std::mem::replace(&mut self.dense_data[dense_idx], value);
            Some(old)
        } else {
            // New entry: push to back of dense arrays
            let dense_idx = self.dense_entities.len();
            self.sparse[entity_idx] = dense_idx + 1;
            self.dense_entities.push(entity);
            self.dense_data.push(value);
            self.dense_ticks.push(tick);
            None
        }
    }

    /// Removes a component for the given entity, preserving dense array packing via swap_remove.
    ///
    /// Returns the removed component value if one existed.
    pub fn remove(&mut self, entity: EntityId) -> Option<T> {
        let entity_idx = entity.index() as usize;
        if entity_idx >= self.sparse.len() {
            return None;
        }

        let dense_slot = self.sparse[entity_idx];
        if dense_slot == 0 {
            return None;
        }

        let dense_idx = dense_slot - 1;
        // Verify entity generation matches
        if self.dense_entities[dense_idx] != entity {
            return None;
        }

        self.sparse[entity_idx] = 0;

        let removed_val = self.dense_data.swap_remove(dense_idx);
        self.dense_entities.swap_remove(dense_idx);
        self.dense_ticks.swap_remove(dense_idx);

        // If we swapped an element from the back to `dense_idx`, update its sparse pointer
        if dense_idx < self.dense_entities.len() {
            let swapped_entity_idx = self.dense_entities[dense_idx].index() as usize;
            self.sparse[swapped_entity_idx] = dense_idx + 1;
        }

        Some(removed_val)
    }

    /// Returns an immutable reference to the component of the given entity.
    #[inline]
    pub fn get(&self, entity: EntityId) -> Option<&T> {
        let entity_idx = entity.index() as usize;
        if entity_idx < self.sparse.len() {
            let dense_slot = self.sparse[entity_idx];
            if dense_slot > 0 {
                let dense_idx = dense_slot - 1;
                if self.dense_entities[dense_idx] == entity {
                    return Some(&self.dense_data[dense_idx]);
                }
            }
        }
        None
    }

    /// Returns a mutable reference to the component of the given entity and updates its tick.
    #[inline]
    pub fn get_mut(&mut self, entity: EntityId, tick: u64) -> Option<&mut T> {
        let entity_idx = entity.index() as usize;
        if entity_idx < self.sparse.len() {
            let dense_slot = self.sparse[entity_idx];
            if dense_slot > 0 {
                let dense_idx = dense_slot - 1;
                if self.dense_entities[dense_idx] == entity {
                    self.dense_ticks[dense_idx] = tick;
                    return Some(&mut self.dense_data[dense_idx]);
                }
            }
        }
        None
    }

    /// Returns the last-modified tick of the given entity's component.
    #[inline]
    pub fn get_tick(&self, entity: EntityId) -> Option<u64> {
        let entity_idx = entity.index() as usize;
        if entity_idx < self.sparse.len() {
            let dense_slot = self.sparse[entity_idx];
            if dense_slot > 0 {
                let dense_idx = dense_slot - 1;
                if self.dense_entities[dense_idx] == entity {
                    return Some(self.dense_ticks[dense_idx]);
                }
            }
        }
        None
    }

    /// Returns `true` if the entity has a component in this set with matching generation.
    #[inline]
    pub fn contains(&self, entity: EntityId) -> bool {
        let entity_idx = entity.index() as usize;
        if entity_idx < self.sparse.len() {
            let dense_slot = self.sparse[entity_idx];
            if dense_slot > 0 {
                return self.dense_entities[dense_slot - 1] == entity;
            }
        }
        false
    }

    /// Returns a slice of all entity IDs currently stored in dense contiguous order.
    #[inline(always)]
    pub fn dense_entities(&self) -> &[EntityId] {
        &self.dense_entities
    }
}

impl<T: Component> Default for SparseSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq, Clone)]
    struct Pos {
        x: i32,
        y: i32,
    }

    #[test]
    fn insert_get_and_remove() {
        let mut set = SparseSet::<Pos>::new();
        let e1 = EntityId::new(1, 1);
        let e2 = EntityId::new(5, 1);
        let e3 = EntityId::new(10, 1);

        assert_eq!(set.insert(e1, Pos { x: 10, y: 20 }, 1), None);
        assert_eq!(set.insert(e2, Pos { x: 50, y: 60 }, 1), None);
        assert_eq!(set.insert(e3, Pos { x: 100, y: 200 }, 1), None);

        assert_eq!(set.dense_entities().len(), 3);
        assert_eq!(set.get(e1), Some(&Pos { x: 10, y: 20 }));
        assert_eq!(set.get(e2), Some(&Pos { x: 50, y: 60 }));
        assert_eq!(set.get(e3), Some(&Pos { x: 100, y: 200 }));
        assert_eq!(set.get_tick(e1), Some(1));

        // Mutate e2
        if let Some(pos) = set.get_mut(e2, 5) {
            pos.x += 1;
        }
        assert_eq!(set.get(e2), Some(&Pos { x: 51, y: 60 }));
        assert_eq!(set.get_tick(e2), Some(5));

        // Remove middle element e2
        assert_eq!(set.remove(e2), Some(Pos { x: 51, y: 60 }));
        assert_eq!(set.dense_entities().len(), 2);
        assert_eq!(set.get(e2), None);
        assert!(!set.contains(e2));

        // Remaining elements are intact
        assert_eq!(set.get(e1), Some(&Pos { x: 10, y: 20 }));
        assert_eq!(set.get(e3), Some(&Pos { x: 100, y: 200 }));
    }

    #[test]
    fn generation_mismatch_returns_none() {
        let mut set = SparseSet::<Pos>::new();
        let e_v1 = EntityId::new(3, 1);
        let e_v2 = EntityId::new(3, 2);

        set.insert(e_v1, Pos { x: 1, y: 2 }, 1);
        assert!(set.contains(e_v1));
        assert!(!set.contains(e_v2));
        assert_eq!(set.get(e_v2), None);
    }
}
