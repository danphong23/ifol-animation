use std::any::TypeId;

/// Unique cache key identifying a compiled query plan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryPlanKey {
    pub query_type_id: TypeId,
    pub component_type_ids: Vec<TypeId>,
    pub registry_revision: u64,
    pub structural_version: u64,
}

impl QueryPlanKey {
    pub fn new(
        query_type_id: TypeId,
        component_type_ids: Vec<TypeId>,
        registry_revision: u64,
        structural_version: u64,
    ) -> Self {
        Self {
            query_type_id,
            component_type_ids,
            registry_revision,
            structural_version,
        }
    }
}
