use crate::error::EcsError;
use crate::storage::Component;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_REGISTRY_ID: AtomicU64 = AtomicU64::new(1);

/// Opaque identifier assigned to a registered component type.
///
/// Registry provenance is part of the identity, so an ID from another
/// registry cannot address the same local numeric slot.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ComponentId {
    registry: u64,
    index: u32,
}

impl ComponentId {
    pub(crate) const fn new(registry: u64, index: u32) -> Self {
        Self { registry, index }
    }

    pub(crate) const fn registry(self) -> u64 {
        self.registry
    }

    /// Returns the local registration index, for diagnostics only.
    pub const fn index(self) -> u32 {
        self.index
    }
}

/// Reflection and runtime metadata for a registered component type.
#[derive(Debug, Clone)]
pub struct ComponentDescriptor {
    pub id: ComponentId,
    pub type_id: TypeId,
    pub type_name: &'static str,
    pub size: usize,
    pub align: usize,
    pub is_world_singleton: bool,
}

/// Registry managing component type registrations, metadata, and revision tracking.
#[derive(Debug, Clone)]
pub struct ComponentRegistry {
    registry_id: u64,
    descriptors: Vec<ComponentDescriptor>,
    type_to_id: HashMap<TypeId, ComponentId>,
    revision: u64,
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentRegistry {
    /// Creates a new empty `ComponentRegistry`.
    pub fn new() -> Self {
        Self {
            registry_id: NEXT_REGISTRY_ID.fetch_add(1, Ordering::Relaxed),
            descriptors: Vec::new(),
            type_to_id: HashMap::new(),
            revision: 0,
        }
    }

    /// Registers a component type `T` into the registry.
    ///
    /// Returns `Ok(ComponentId)` on success, or `Err(EcsError::DuplicateComponent)` if already registered.
    pub fn register<T: Component>(&mut self) -> Result<ComponentId, EcsError> {
        self.register_internal::<T>(false)
    }

    /// Registers a component type `T` designated as a world singleton on `WORLD_ENTITY`.
    pub fn register_world_singleton<T: Component>(&mut self) -> Result<ComponentId, EcsError> {
        self.register_internal::<T>(true)
    }

    pub(crate) fn ensure_world_singleton<T: Component>(&mut self) -> Result<(), EcsError> {
        if let Some(id) = self.get_id::<T>() {
            if let Some(descriptor) = self.descriptors.get_mut(id.index() as usize) {
                descriptor.is_world_singleton = true;
            }
            return Ok(());
        }
        self.register_world_singleton::<T>().map(|_| ())
    }

    fn register_internal<T: Component>(
        &mut self,
        is_world_singleton: bool,
    ) -> Result<ComponentId, EcsError> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        if self.type_to_id.contains_key(&type_id) {
            return Err(EcsError::DuplicateComponent(type_name));
        }

        let id = ComponentId::new(self.registry_id, self.descriptors.len() as u32);
        let descriptor = ComponentDescriptor {
            id,
            type_id,
            type_name,
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
            is_world_singleton,
        };

        self.descriptors.push(descriptor);
        self.type_to_id.insert(type_id, id);
        self.revision = self.revision.wrapping_add(1);

        Ok(id)
    }

    /// Returns the `ComponentId` for type `T` if registered.
    #[inline]
    pub fn get_id<T: Component>(&self) -> Option<ComponentId> {
        self.type_to_id.get(&TypeId::of::<T>()).copied()
    }

    /// Returns the `ComponentId` for a given `TypeId` if registered.
    #[inline]
    pub fn get_id_by_type_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.type_to_id.get(&type_id).copied()
    }

    /// Returns the descriptor for the given `ComponentId` if valid.
    #[inline]
    pub fn descriptor(&self, id: ComponentId) -> Option<&ComponentDescriptor> {
        (id.registry() == self.registry_id)
            .then(|| self.descriptors.get(id.index() as usize))
            .flatten()
    }

    /// Returns the current monotonic registration revision.
    #[inline(always)]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns the total number of registered components.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    /// Returns `true` if no components are registered.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Pos(f32, f32);
    #[derive(Debug, Clone, PartialEq)]
    struct Vel(f32, f32);

    #[test]
    fn component_registration_and_revision() {
        let mut reg = ComponentRegistry::new();
        assert_eq!(reg.revision(), 0);

        let id_pos = reg.register::<Pos>().unwrap();
        assert_eq!(id_pos.index(), 0);
        assert_eq!(reg.revision(), 1);

        let id_vel = reg.register::<Vel>().unwrap();
        assert_eq!(id_vel.index(), 1);
        assert_eq!(reg.revision(), 2);

        // Duplicate registration fails closed
        assert_eq!(
            reg.register::<Pos>(),
            Err(EcsError::DuplicateComponent(std::any::type_name::<Pos>()))
        );

        assert_eq!(reg.get_id::<Pos>(), Some(id_pos));
        assert_eq!(reg.get_id::<Vel>(), Some(id_vel));
    }
}
