use crate::error::EcsError;
use crate::registry::{ComponentId, ComponentRegistry};
use crate::world::World;

/// Execution condition evaluated before invoking a system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunCondition {
    /// Always run the system unconditionally.
    Always,
    /// Only run if the root `WORLD_ENTITY` has the specified component.
    WorldHas(ComponentId, &'static str),
    /// Run only if all sub-conditions evaluate to true.
    All(Vec<RunCondition>),
    /// Run if any sub-condition evaluates to true.
    Any(Vec<RunCondition>),
}

impl RunCondition {
    pub(crate) fn validate(&self, components: &ComponentRegistry) -> Result<(), EcsError> {
        match self {
            Self::Always => Ok(()),
            Self::WorldHas(id, _) => components
                .descriptor(*id)
                .map(|_| ())
                .ok_or_else(|| EcsError::ComponentIdNotRegistered(format!("{id:?}"))),
            Self::All(conditions) | Self::Any(conditions) => conditions
                .iter()
                .try_for_each(|condition| condition.validate(components)),
        }
    }

    /// Evaluates this run condition against current world state.
    ///
    /// Returns `Ok(())` if condition passes, or `Err(reason)` if failed.
    pub fn evaluate(&self, world: &World) -> Result<(), String> {
        match self {
            Self::Always => Ok(()),
            Self::WorldHas(comp_id, name) => {
                if world.has_world_component_by_id(*comp_id) {
                    Ok(())
                } else {
                    Err(format!("Missing required world singleton '{name}'"))
                }
            }
            Self::All(conditions) => {
                for cond in conditions {
                    cond.evaluate(world)?;
                }
                Ok(())
            }
            Self::Any(conditions) => {
                if conditions.is_empty() {
                    return Err("No conditions supplied".to_string());
                }
                let mut last_err = String::new();
                for cond in conditions {
                    match cond.evaluate(world) {
                        Ok(()) => return Ok(()),
                        Err(e) => last_err = e,
                    }
                }
                Err(format!("No conditions satisfied: {last_err}"))
            }
        }
    }
}
