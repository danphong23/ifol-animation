use std::any::TypeId;

/// Unique cache key identifying a compiled query plan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryPlanKey {
    pub component_type_ids: Vec<TypeId>,
    pub registry_revision: u64,
    pub structural_version: u64,
}

impl QueryPlanKey {
    pub fn new(
        component_type_ids: Vec<TypeId>,
        registry_revision: u64,
        structural_version: u64,
    ) -> Self {
        Self {
            component_type_ids,
            registry_revision,
            structural_version,
        }
    }
}
