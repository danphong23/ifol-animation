use crate::entity::EntityId;
use crate::error::EcsError;

/// Manages entity lifecycle, generations, slot allocation, and alive tracking.
#[derive(Debug, Clone)]
pub struct EntityManager {
    generations: Vec<u32>,
    alive_flags: Vec<bool>,
    free_indices: Vec<u32>,
    alive_count: usize,
}

impl EntityManager {
    /// Creates a new `EntityManager` with the root `WORLD` entity pre-spawned.
    pub fn new() -> Self {
        // Slot 0 is reserved for EntityId::WORLD (generation 1, alive).
        let generations = vec![EntityId::WORLD.generation()];
        let alive_flags = vec![true];
        Self {
            generations,
            alive_flags,
            free_indices: Vec::new(),
            alive_count: 1, // WORLD entity is always alive
        }
    }

    /// Spawns a new unique `EntityId`.
    ///
    /// Reuses free slots if available; otherwise allocates a new index at generation 1.
    pub fn spawn(&mut self) -> EntityId {
        if let Some(index) = self.free_indices.pop() {
            let idx = index as usize;
            let gen_val = self.generations[idx];
            self.alive_flags[idx] = true;
            self.alive_count += 1;
            EntityId::new(index, gen_val)
        } else {
            let index = self.generations.len() as u32;
            let gen_val = 1;
            self.generations.push(gen_val);
            self.alive_flags.push(true);
            self.alive_count += 1;
            EntityId::new(index, gen_val)
        }
    }

    /// Despawns an entity, incrementing its generation and recycling its slot index.
    ///
    /// Returns `Err(EcsError::EntityNotFound)` if the entity is not alive.
    pub fn despawn(&mut self, entity: EntityId) -> Result<(), EcsError> {
        let index = entity.index() as usize;
        if index >= self.generations.len() {
            return Err(EcsError::EntityNotFound(entity));
        }

        // Cannot despawn the root WORLD entity
        if entity.is_world() {
            return Err(EcsError::EntityNotFound(entity));
        }

        if !self.alive_flags[index] || self.generations[index] != entity.generation() {
            return Err(EcsError::EntityNotFound(entity));
        }

        // Mark as dead and increment generation to invalidate existing handles
        self.alive_flags[index] = false;
        self.generations[index] = self.generations[index].wrapping_add(1);
        self.free_indices.push(entity.index());
        self.alive_count -= 1;
        Ok(())
    }

    /// Returns `true` if the entity is currently alive with matching generation.
    #[inline(always)]
    pub fn is_alive(&self, entity: EntityId) -> bool {
        let index = entity.index() as usize;
        if index < self.generations.len() {
            self.alive_flags[index] && self.generations[index] == entity.generation()
        } else {
            false
        }
    }

    /// Validates an entity ID, returning a typed `EcsError` if invalid or dead.
    pub fn validate(&self, entity: EntityId) -> Result<(), EcsError> {
        let index = entity.index() as usize;
        if index >= self.generations.len() {
            return Err(EcsError::EntityNotFound(entity));
        }

        if !self.alive_flags[index] {
            return Err(EcsError::ForgedEntityId(entity));
        }

        if self.generations[index] != entity.generation() {
            return Err(EcsError::EntityNotFound(entity));
        }

        Ok(())
    }

    /// Returns the total number of currently alive entities (including WORLD).
    #[inline(always)]
    pub fn alive_count(&self) -> usize {
        self.alive_count
    }

    /// Returns the total capacity of allocated entity slots.
    #[inline(always)]
    pub fn total_slots(&self) -> usize {
        self.generations.len()
    }

    /// Returns an iterator over all currently alive `EntityId`s.
    pub fn iter_alive(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.generations
            .iter()
            .enumerate()
            .filter_map(|(idx, &gen_val)| {
                if self.alive_flags[idx] {
                    Some(EntityId::new(idx as u32, gen_val))
                } else {
                    None
                }
            })
    }
}

impl Default for EntityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_and_despawn_recycling() {
        let mut mgr = EntityManager::new();
        assert!(mgr.is_alive(EntityId::WORLD));
        assert_eq!(mgr.alive_count(), 1);

        let e1 = mgr.spawn();
        assert_eq!(e1.index(), 1);
        assert_eq!(e1.generation(), 1);
        assert!(mgr.is_alive(e1));
        assert_eq!(mgr.alive_count(), 2);

        let e2 = mgr.spawn();
        assert_eq!(e2.index(), 2);
        assert_eq!(e2.generation(), 1);
        assert_eq!(mgr.alive_count(), 3);

        // Despawn e1
        assert!(mgr.despawn(e1).is_ok());
        assert!(!mgr.is_alive(e1));
        assert_eq!(mgr.alive_count(), 2);

        // Trying to despawn e1 again fails closed
        assert_eq!(mgr.despawn(e1), Err(EcsError::EntityNotFound(e1)));

        // Spawn e3 should reuse slot 1 with generation 2
        let e3 = mgr.spawn();
        assert_eq!(e3.index(), 1);
        assert_eq!(e3.generation(), 2);
        assert!(mgr.is_alive(e3));
        assert!(!mgr.is_alive(e1)); // Stale handle is still dead
    }

    #[test]
    fn cannot_despawn_world_entity() {
        let mut mgr = EntityManager::new();
        assert_eq!(
            mgr.despawn(EntityId::WORLD),
            Err(EcsError::EntityNotFound(EntityId::WORLD))
        );
    }

    #[test]
    fn forged_id_rejection() {
        let mut mgr = EntityManager::new();
        let e1 = mgr.spawn();
        mgr.despawn(e1).unwrap();

        // Slot 1 is now dead with next gen = 2
        // If someone forges an EntityId(1, 2) before it is spawned, validate should reject it as ForgedEntityId
        let forged = EntityId::new(1, 2);
        assert!(!mgr.is_alive(forged));
        assert_eq!(mgr.validate(forged), Err(EcsError::ForgedEntityId(forged)));

        // Index out of bounds
        let out_of_bounds = EntityId::new(999, 1);
        assert_eq!(
            mgr.validate(out_of_bounds),
            Err(EcsError::EntityNotFound(out_of_bounds))
        );
    }
}
