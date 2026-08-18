use crate::error::EcsError;
use crate::registry::{PhaseId, PhaseRegistry};
use std::collections::{HashMap, HashSet, VecDeque};

/// Phase graph compiler compiling registered phases and topological dependencies.
pub struct PhaseGraph;

impl PhaseGraph {
    /// Performs Kahn's topological sort on all phases in the registry.
    ///
    /// Returns an ordered list of `PhaseId`s, or returns `Err(EcsError)` if a cycle or missing dependency is found.
    pub fn compile_order(registry: &PhaseRegistry) -> Result<Vec<PhaseId>, EcsError> {
        let phases = registry.phases();
        if phases.is_empty() {
            return Ok(Vec::new());
        }

        // Validate dependencies and calculate in-degrees
        let mut in_degrees: HashMap<PhaseId, usize> = HashMap::new();
        let mut adj_list: HashMap<PhaseId, Vec<PhaseId>> = HashMap::new();

        for (id, node) in phases {
            in_degrees.entry(id.clone()).or_insert(0);
            adj_list.entry(id.clone()).or_default();

            for before in node.before() {
                if !phases.contains_key(before) {
                    return Err(EcsError::MissingPhaseDependency {
                        phase: id.to_string(),
                        dependency: before.to_string(),
                    });
                }
                *in_degrees.entry(before.clone()).or_insert(0) += 1;
                adj_list.entry(id.clone()).or_default().push(before.clone());
            }
        }

        // Collect all nodes with zero in-degree
        let mut zero_in_degree: Vec<PhaseId> = in_degrees
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        // Sort zero in-degree nodes for deterministic tie-breaking
        zero_in_degree.sort();
        let mut queue: VecDeque<PhaseId> = VecDeque::from(zero_in_degree);

        let mut sorted_order: Vec<PhaseId> = Vec::with_capacity(phases.len());
        let mut visited: HashSet<PhaseId> = HashSet::new();

        while let Some(current) = queue.pop_front() {
            sorted_order.push(current.clone());
            visited.insert(current.clone());

            if let Some(neighbors) = adj_list.get(&current) {
                let mut ready_neighbors = Vec::new();
                for neighbor in neighbors {
                    let deg = in_degrees.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 && !visited.contains(neighbor) {
                        ready_neighbors.push(neighbor.clone());
                    }
                }
                ready_neighbors.sort();
                for n in ready_neighbors {
                    queue.push_back(n);
                }
            }
        }

        if sorted_order.len() != phases.len() {
            let unvisited: Vec<String> = phases
                .keys()
                .filter(|id| !visited.contains(id))
                .map(|id| id.to_string())
                .collect();
            return Err(EcsError::PhaseCycleDetected(unvisited.join(", ")));
        }

        Ok(sorted_order)
    }
}
