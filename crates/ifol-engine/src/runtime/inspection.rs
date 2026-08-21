use super::EngineRuntime;
use crate::state::EngineState;

impl EngineRuntime {
    #[inline]
    pub fn provider_manager(&self) -> &crate::provider::ProviderManager {
        &self.provider_manager
    }
    #[inline]
    pub fn command_registry(&self) -> &crate::registration::CommandRegistry {
        &self.command_registry
    }
    #[inline]
    pub fn state(&self) -> EngineState {
        self.state
    }
    #[inline]
    pub fn revision(&self) -> u64 {
        self.revision
    }
    #[inline]
    pub fn package_lock(&self) -> &crate::package::PackageLock {
        &self.package_lock
    }
    #[inline]
    pub fn schema_registry(&self) -> &crate::scene::SchemaRegistry {
        &self.schemas
    }
    #[inline]
    pub fn migration_registry(&self) -> &crate::scene::MigrationRegistry {
        &self.migrations
    }
    #[inline]
    pub fn namespace_registry(&self) -> &crate::namespace::NamespaceRegistry {
        &self.namespaces
    }
    pub fn active_scene(&self) -> Option<&crate::scene::SceneId> {
        self.active_scene.as_ref()
    }
    pub fn active_scene_entity_count(&self) -> usize {
        self.active_scene_entities.len()
    }
}
