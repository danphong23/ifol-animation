use super::EngineRuntime;
use crate::error::EngineError;
use crate::state::EngineState;
use std::collections::BTreeSet;

impl EngineRuntime {
    /// Loads one validated scene document into the ECS world atomically.
    pub fn load_scene(
        &mut self,
        document: &crate::scene::SceneDocument,
    ) -> Result<crate::scene::SceneLoadResult, EngineError> {
        self.load_scene_as(crate::scene::SceneId::entry(), document)
    }

    /// Loads a scene and replaces the previous active scene after successful validation.
    pub fn load_scene_as(
        &mut self,
        scene_id: crate::scene::SceneId,
        document: &crate::scene::SceneDocument,
    ) -> Result<crate::scene::SceneLoadResult, EngineError> {
        self.require_state(EngineState::Ready, "load_scene")?;
        let result = crate::scene::SceneLoader::load_scene(
            self.ecs.world_mut(),
            document,
            &self.schemas,
            &self.migrations,
        )?;
        let new_entities: BTreeSet<_> = result.key_to_entity.values().copied().collect();
        for entity in self.active_scene_entities.iter().copied() {
            if let Err(error) = self.ecs.despawn(entity) {
                self.state = EngineState::Faulted;
                return Err(EngineError::Ecs(error));
            }
        }
        self.active_scene = Some(scene_id.clone());
        self.active_scene_entities = new_entities;
        self.revision = self.revision.wrapping_add(1);
        Ok(crate::scene::SceneLoadResult {
            scene_id: Some(scene_id),
            ..result
        })
    }

    /// Removes the active scene while preserving registrations and world resources.
    pub fn clear_scene(&mut self) -> Result<bool, EngineError> {
        self.require_state(EngineState::Ready, "clear_scene")?;
        let had_scene = self.active_scene.is_some();
        for entity in self.active_scene_entities.iter().copied() {
            self.ecs.despawn(entity)?;
        }
        self.active_scene = None;
        self.active_scene_entities.clear();
        if had_scene {
            self.revision = self.revision.wrapping_add(1);
        }
        Ok(had_scene)
    }
}
