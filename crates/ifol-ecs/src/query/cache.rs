use crate::entity::EntityId;
use crate::query::plan::QueryPlanKey;
use std::collections::HashMap;

/// Cache storing matching entity candidate lists indexed by `QueryPlanKey`.
#[derive(Default, Debug, Clone)]
pub struct QueryPlanCache {
    cache: HashMap<QueryPlanKey, Vec<EntityId>>,
    hits: usize,
    misses: usize,
}

impl QueryPlanCache {
    /// Creates a new empty `QueryPlanCache`.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// Attempts to retrieve matching entities from cache.
    pub fn get(&mut self, key: &QueryPlanKey) -> Option<&[EntityId]> {
        if let Some(entities) = self.cache.get(key) {
            self.hits += 1;
            Some(entities.as_slice())
        } else {
            self.misses += 1;
            None
        }
    }

    /// Stores matching entity candidates for the given key.
    pub fn insert(&mut self, key: QueryPlanKey, entities: Vec<EntityId>) {
        self.cache.insert(key, entities);
    }

    /// Clears the query plan cache.
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Returns cache statistics `(hits, misses)`.
    #[inline(always)]
    pub fn stats(&self) -> (usize, usize) {
        (self.hits, self.misses)
    }
}
