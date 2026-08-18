use crate::error::EcsError;
use crate::schedule::phase::PhaseId;
use std::collections::{HashMap, VecDeque};

/// Describes dependency constraints for a phase in the schedule.
#[derive(Debug, Clone)]
pub struct PhaseConfig {
    pub id: PhaseId,
    pub before: Vec<PhaseId>,
    pub after: Vec<PhaseId>,
}

impl PhaseConfig {
    pub fn new(id: PhaseId) -> Self {
        Self {
            id,
            before: Vec::new(),
            after: Vec::new(),
        }
    }
}

/// Sorts phases in topological order respecting `before` and `after` constraints.
///
/// Returns `Err(EcsError::PhaseCycleDetected)` if a dependency cycle is detected.
/// Returns `Err(EcsError::MissingPhaseDependency)` if a required phase is not registered.
pub fn sort_phases(phases: &[PhaseConfig]) -> Result<Vec<PhaseId>, EcsError> {
    let mut phase_map: HashMap<&PhaseId, &PhaseConfig> = HashMap::new();
    for config in phases {
        if phase_map.insert(&config.id, config).is_some() {
            return Err(EcsError::DuplicatePhase(config.id.to_string()));
        }
    }

    // Build adjacency graph: A -> B means A must execute before B
    let mut adj: HashMap<&PhaseId, Vec<&PhaseId>> = HashMap::new();
    let mut in_degree: HashMap<&PhaseId, usize> = HashMap::new();

    for config in phases {
        adj.entry(&config.id).or_default();
        in_degree.entry(&config.id).or_insert(0);
    }

    for config in phases {
        let from_id = &config.id;

        // "before" constraint: from_id must run before target
        for target in &config.before {
            if !phase_map.contains_key(target) {
                return Err(EcsError::MissingPhaseDependency(
                    from_id.to_string(),
                    target.to_string(),
                ));
            }
            adj.get_mut(from_id).unwrap().push(target);
            *in_degree.get_mut(target).unwrap() += 1;
        }

        // "after" constraint: target must run before from_id
        for target in &config.after {
            if !phase_map.contains_key(target) {
                return Err(EcsError::MissingPhaseDependency(
                    from_id.to_string(),
                    target.to_string(),
                ));
            }
            adj.get_mut(target).unwrap().push(from_id);
            *in_degree.get_mut(from_id).unwrap() += 1;
        }
    }

    // Kahn's algorithm: queue nodes with in_degree == 0
    // To ensure deterministic ordering, sort zero in-degree nodes by registration order
    let mut queue = VecDeque::new();
    for config in phases {
        if in_degree[&config.id] == 0 {
            queue.push_back(&config.id);
        }
    }

    let mut sorted = Vec::new();

    while let Some(current) = queue.pop_front() {
        sorted.push((*current).clone());

        if let Some(neighbors) = adj.get(current) {
            for &neighbor in neighbors {
                let degree = in_degree.get_mut(neighbor).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(neighbor);
                }
            }
        }
    }

    if sorted.len() != phases.len() {
        // Collect unresolved cyclic phases
        let unvisited: Vec<String> = phases
            .iter()
            .filter(|c| in_degree[&c.id] > 0)
            .map(|c| c.id.to_string())
            .collect();
        return Err(EcsError::PhaseCycleDetected(unvisited.join(" <-> ")));
    }

    Ok(sorted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_dependency_sort() {
        let phases = vec![
            PhaseConfig {
                id: PhaseId::Update,
                before: vec![PhaseId::RenderSubmit],
                after: vec![PhaseId::PreUpdate],
            },
            PhaseConfig {
                id: PhaseId::PreUpdate,
                before: vec![],
                after: vec![],
            },
            PhaseConfig {
                id: PhaseId::RenderSubmit,
                before: vec![],
                after: vec![],
            },
        ];

        let sorted = sort_phases(&phases).unwrap();
        assert_eq!(
            sorted,
            vec![PhaseId::PreUpdate, PhaseId::Update, PhaseId::RenderSubmit]
        );
    }

    #[test]
    fn cycle_detection_reports_error() {
        let phases = vec![
            PhaseConfig {
                id: PhaseId::custom("PhaseA"),
                before: vec![PhaseId::custom("PhaseB")],
                after: vec![],
            },
            PhaseConfig {
                id: PhaseId::custom("PhaseB"),
                before: vec![PhaseId::custom("PhaseA")],
                after: vec![],
            },
        ];

        let err = sort_phases(&phases).unwrap_err();
        assert!(matches!(err, EcsError::PhaseCycleDetected(_)));
    }

    #[test]
    fn missing_dependency_reports_error() {
        let phases = vec![PhaseConfig {
            id: PhaseId::PreUpdate,
            before: vec![PhaseId::custom("NonExistent")],
            after: vec![],
        }];

        let err = sort_phases(&phases).unwrap_err();
        assert!(matches!(err, EcsError::MissingPhaseDependency(_, _)));
    }
}
