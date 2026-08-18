mod support;

use ifol_ecs::error::EcsError;
use ifol_ecs::registry::{PhaseId, PhaseRegistry};
use ifol_ecs::schedule::PhaseGraph;

#[test]
fn slice06_phase_graph_topological_sort_and_cycle_detection() {
    let mut reg = PhaseRegistry::new();

    // 1. Register 5 phases
    let p_pre = PhaseId::new("prepare");
    let p_up = PhaseId::new("simulate");
    let p_post = PhaseId::new("finalize");
    let p_prep = PhaseId::new("graph");
    let p_sub = PhaseId::new("submit");

    reg.register_phase(p_sub.clone()).unwrap();
    reg.register_phase(p_post.clone()).unwrap();
    reg.register_phase(p_up.clone()).unwrap();
    reg.register_phase(p_pre.clone()).unwrap();
    reg.register_phase(p_prep.clone()).unwrap();

    // Add edges
    reg.add_phase_edge(&p_pre, &p_up).unwrap();
    reg.add_phase_edge(&p_up, &p_post).unwrap();
    reg.add_phase_edge(&p_post, &p_prep).unwrap();
    reg.add_phase_edge(&p_prep, &p_sub).unwrap();

    // 2. Compile order
    let order = PhaseGraph::compile_order(&reg).unwrap();
    assert_eq!(
        order,
        vec![
            p_pre.clone(),
            p_up.clone(),
            p_post.clone(),
            p_prep.clone(),
            p_sub.clone(),
        ]
    );

    // 3. 2-Node Direct Cycle: A <-> B
    let mut cycle_reg = PhaseRegistry::new();
    let p_a = PhaseId::new("phase.a");
    let p_b = PhaseId::new("phase.b");

    cycle_reg.register_phase(p_a.clone()).unwrap();
    cycle_reg.register_phase(p_b.clone()).unwrap();
    cycle_reg.add_phase_edge(&p_a, &p_b).unwrap();
    cycle_reg.add_phase_edge(&p_b, &p_a).unwrap();

    let cycle_res = PhaseGraph::compile_order(&cycle_reg);
    assert!(matches!(cycle_res, Err(EcsError::PhaseCycleDetected(_))));

    // 4. Missing Dependency
    let mut missing_reg = PhaseRegistry::new();
    missing_reg.register_phase(p_a.clone()).unwrap();
    // PhaseB is not registered
    let missing_edge = missing_reg.add_phase_edge(&p_a, &p_b);
    assert_eq!(
        missing_edge,
        Err(EcsError::PhaseNotFound("phase.b".to_string()))
    );
}
