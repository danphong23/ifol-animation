use super::flatten::{FlatRenderNode, FlatRenderPlan, GraphDependency, GraphFlattenError};
use super::graph::RenderGraph;
use super::usage::usages_conflict;
use super::{RenderNode, RenderNodePool, ResourceUsage};
use crate::resources::handle::RenderNodeId;
use std::collections::{HashMap, HashSet};

impl RenderGraph {
    /// Làm phẳng logical graph theo thứ tự thực thi bottom-up: node con của
    /// `SubGraph` xuất hiện trước node composite của chính subgraph.
    pub fn flatten(&self, pool: &RenderNodePool) -> Result<FlatRenderPlan, GraphFlattenError> {
        let mut plan = FlatRenderPlan::default();
        let mut active = Vec::new();
        let mut usage_map = HashMap::new();
        self.flatten_into(pool, &mut plan, &mut active, Vec::new(), &mut usage_map)?;
        let mut dependencies = Vec::new();
        self.collect_dependencies(pool, &mut dependencies)?;
        Self::apply_dependencies(&mut plan, &usage_map, &dependencies)?;
        Ok(plan)
    }

    fn apply_dependencies(
        plan: &mut FlatRenderPlan,
        usage_map: &HashMap<RenderNodeId, Vec<ResourceUsage>>,
        dependencies: &[GraphDependency],
    ) -> Result<(), GraphFlattenError> {
        if plan.nodes.len() < 2 {
            return Ok(());
        }
        let positions = plan
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.node_id, index))
            .collect::<HashMap<_, _>>();
        let mut edges = vec![Vec::new(); plan.nodes.len()];
        let mut indegree = vec![0usize; plan.nodes.len()];
        let mut edge_set = HashSet::new();
        let mut add_edge = |before: usize, after: usize| {
            if edge_set.insert((before, after)) {
                edges[before].push(after);
                indegree[after] += 1;
            }
        };
        for dependency in dependencies {
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

        for before in 0..plan.nodes.len() {
            for after in (before + 1)..plan.nodes.len() {
                let before_usages = usage_map
                    .get(&plan.nodes[before].node_id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let after_usages = usage_map
                    .get(&plan.nodes[after].node_id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                if before_usages.iter().any(|left| {
                    after_usages
                        .iter()
                        .any(|right| usages_conflict(left, right))
                }) {
                    add_edge(before, after);
                }
            }
        }

        let original = plan.nodes.clone();
        let mut ordered = Vec::with_capacity(original.len());
        let mut emitted = vec![false; original.len()];
        while ordered.len() < original.len() {
            let Some(index) =
                (0..original.len()).find(|&index| !emitted[index] && indegree[index] == 0)
            else {
                let cycle = original
                    .iter()
                    .find(|node| !emitted[positions[&node.node_id]])
                    .map(|node| node.node_id)
                    .unwrap_or(RenderNodeId(0));
                return Err(GraphFlattenError::Cycle(cycle));
            };
            emitted[index] = true;
            ordered.push(original[index].clone());
            for &next in &edges[index] {
                indegree[next] -= 1;
            }
        }
        plan.nodes = ordered;
        Ok(())
    }

    fn collect_dependencies(
        &self,
        pool: &RenderNodePool,
        dependencies: &mut Vec<GraphDependency>,
    ) -> Result<(), GraphFlattenError> {
        let node_set: HashSet<_> = self.node_ids.iter().copied().collect();
        for dependency in &self.dependencies {
            if !node_set.contains(&dependency.before) {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(
                    dependency.before,
                ));
            }
            if !node_set.contains(&dependency.after) {
                return Err(GraphFlattenError::DependencyNodeOutsideGraph(
                    dependency.after,
                ));
            }
            dependencies.push(*dependency);
        }
        for &node_id in &self.node_ids {
            let node = pool
                .get(node_id)
                .ok_or(GraphFlattenError::MissingNode(node_id))?;
            if let RenderNode::SubGraph { graph, .. } = node {
                graph.collect_dependencies(pool, dependencies)?;
            }
        }
        Ok(())
    }

    fn flatten_into(
        &self,
        pool: &RenderNodePool,
        plan: &mut FlatRenderPlan,
        active: &mut Vec<RenderNodeId>,
        parent_path: Vec<RenderNodeId>,
        usage_map: &mut HashMap<RenderNodeId, Vec<ResourceUsage>>,
    ) -> Result<(), GraphFlattenError> {
        for &node_id in &self.node_ids {
            if active.contains(&node_id) {
                return Err(GraphFlattenError::Cycle(node_id));
            }
            let node = pool
                .get(node_id)
                .ok_or(GraphFlattenError::MissingNode(node_id))?;
            usage_map.insert(node_id, self.effective_resource_usages(node_id, pool));
            let mut path = parent_path.clone();
            path.push(node_id);
            if let RenderNode::SubGraph { graph, .. } = node {
                active.push(node_id);
                graph.flatten_into(pool, plan, active, path.clone(), usage_map)?;
                active.pop();
            }
            plan.nodes.push(FlatRenderNode { node_id, path });
        }
        Ok(())
    }
}
