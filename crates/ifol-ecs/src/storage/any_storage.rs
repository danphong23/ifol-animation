use crate::entity::EntityId;
use crate::storage::component::Component;
use crate::storage::sparse_set::SparseSet;
use std::any::Any;

/// Type-erased storage interface used by `World` to manage component lifecycles.
pub trait AnyStorage: 'static + Send + Sync {
    /// Casts this storage to `&dyn Any`.
    fn as_any(&self) -> &dyn Any;

    /// Casts this storage to `&mut dyn Any`.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Removes an entity's component if it is present.
    fn remove_entity(&mut self, entity: EntityId);

    /// Checks if the entity is stored in this storage.
    fn contains_entity(&self, entity: EntityId) -> bool;
}

impl<T: Component> AnyStorage for SparseSet<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn remove_entity(&mut self, entity: EntityId) {
        self.remove(entity);
    }

    fn contains_entity(&self, entity: EntityId) -> bool {
        self.contains(entity)
    }
}
