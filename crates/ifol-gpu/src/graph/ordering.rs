use super::flatten::GraphFlattenError;
use super::render_graph::RenderGraph;
use super::usage::usages_conflict;
use super::RenderNodePool;
use crate::resources::handle::RenderNodeId;
use std::collections::{HashMap, HashSet};

impl RenderGraph {
    /// Trả về thứ tự của các node trực tiếp thuộc graph sau khi áp dụng
    /// dependency. Declaration order là tie-breaker ổn định.
    pub fn ordered_node_ids(
        &self,
        pool: &RenderNodePool,
    ) -> Result<Vec<RenderNodeId>, GraphFlattenError> {
        let positions = self
            .node_ids
            .iter()
            .enumerate()
            .map(|(index, &id)| (id, index))
            .collect::<HashMap<_, _>>();
        for &node_id in &self.node_ids {
            if pool.get(node_id).is_none() {
                return Err(GraphFlattenError::MissingNode(node_id));
            }
        }
        let mut edges = vec![Vec::new(); self.node_ids.len()];
        let mut indegree = vec![0usize; self.node_ids.len()];
        let mut edge_set = HashSet::new();
        let mut add_edge = |before: usize, after: usize| {
            if edge_set.insert((before, after)) {
                edges[before].push(after);
                indegree[after] += 1;
            }
        };
        for dependency in &self.dependencies {
            let Some(&before) = positions.get(&dependency.before) else {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(
                    dependency.before,
                ));
            };
            let Some(&after) = positions.get(&dependency.after) else {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(
                    dependency.after,
                ));
            };
            add_edge(before, after);
        }

        // Declaration order is the stable tie-breaker for implicit hazards.
        // A later node cannot observe/write the same resource before the earlier
        // node when at least one side writes it.
        for before in 0..self.node_ids.len() {
            for after in (before + 1)..self.node_ids.len() {
                let before_usages = self.effective_resource_usages(self.node_ids[before], pool);
                let after_usages = self.effective_resource_usages(self.node_ids[after], pool);
                let conflict = before_usages.iter().any(|left| {
                    after_usages
                        .iter()
                        .any(|right| usages_conflict(left, right))
                });
                if conflict {
                    add_edge(before, after);
                }
            }
        }

        let mut ordered = Vec::with_capacity(self.node_ids.len());
        let mut emitted = vec![false; self.node_ids.len()];
        while ordered.len() < self.node_ids.len() {
            let Some(index) =
                (0..self.node_ids.len()).find(|&index| !emitted[index] && indegree[index] == 0)
            else {
                let cycle = self
                    .node_ids
                    .iter()
                    .enumerate()
                    .find(|(index, _)| !emitted[*index])
                    .map(|(_, &id)| id)
                    .unwrap_or(RenderNodeId(0));
                return Err(GraphFlattenError::Cycle(cycle));
            };
            emitted[index] = true;
            ordered.push(self.node_ids[index]);
            for &next in &edges[index] {
                indegree[next] -= 1;
            }
        }
        Ok(ordered)
    }
}
