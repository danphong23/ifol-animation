use std::fmt;

/// A 64-bit generational entity identifier.
///
/// Combines a 32-bit slot `index` and a 32-bit `generation` counter.
/// When an entity is despawned, its generation is incremented to prevent
/// dangling references from accessing recycled slots.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId {
    index: u32,
    generation: u32,
}

impl EntityId {
    /// Predefined constant entity identifier representing the root World Entity.
    ///
    /// Used for storing global / singleton components with O(1) lookup.
    pub const WORLD: Self = Self {
        index: 0,
        generation: 1,
    };

    /// Creates a new `EntityId` with the given index and generation.
    #[inline(always)]
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Returns the slot index of this entity.
    #[inline(always)]
    pub const fn index(self) -> u32 {
        self.index
    }

    /// Returns the generation count of this entity.
    #[inline(always)]
    pub const fn generation(self) -> u32 {
        self.generation
    }

    /// Returns `true` if this entity is the root World Entity.
    #[inline(always)]
    pub const fn is_world(self) -> bool {
        self.index == Self::WORLD.index && self.generation == Self::WORLD.generation
    }
}

impl fmt::Debug for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_world() {
            write!(f, "EntityId(WORLD)")
        } else {
            write!(f, "EntityId({}v{})", self.index, self.generation)
        }
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_world() {
            write!(f, "eWORLD")
        } else {
            write!(f, "e{}v{}", self.index, self.generation)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_id_basics() {
        let e = EntityId::new(42, 3);
        assert_eq!(e.index(), 42);
        assert_eq!(e.generation(), 3);
        assert!(!e.is_world());
        assert_eq!(format!("{:?}", e), "EntityId(42v3)");
        assert_eq!(format!("{}", e), "e42v3");
    }

    #[test]
    fn world_entity_constant() {
        let w = EntityId::WORLD;
        assert_eq!(w.index(), 0);
        assert_eq!(w.generation(), 1);
        assert!(w.is_world());
        assert_eq!(format!("{:?}", w), "EntityId(WORLD)");
        assert_eq!(format!("{}", w), "eWORLD");
    }
}
