mod support;

use ifol_ecs::error::EcsError;
use ifol_ecs::registry::{ComponentRegistry, PhaseId, PhaseRegistry, SystemRegistry};
use ifol_ecs::system::{AccessDescriptor, FunctionSystem};
use support::{Position, Velocity};

#[test]
fn slice05_registry_transactional_commit_and_revisions() {
    // 1. ComponentRegistry
    let mut comp_reg = ComponentRegistry::new();
    let id_pos = comp_reg.register::<Position>().unwrap();
    let id_vel = comp_reg.register::<Velocity>().unwrap();
    assert_eq!(comp_reg.revision(), 2);

    // Duplicate component registration
    assert_eq!(
        comp_reg.register::<Position>(),
        Err(EcsError::DuplicateComponent(
            std::any::type_name::<Position>()
        ))
    );

    // 2. PhaseRegistry
    let mut phase_reg = PhaseRegistry::new();
    let prepare = PhaseId::new("prepare");
    let simulate = PhaseId::new("simulate");
    phase_reg.register_phase(prepare.clone()).unwrap();
    phase_reg.register_phase(simulate.clone()).unwrap();
    assert_eq!(phase_reg.revision(), 2);

    assert_eq!(
        phase_reg.register_phase(PhaseId::new("")),
        Err(EcsError::InvalidPhaseId)
    );

    // Duplicate phase registration
    assert_eq!(
        phase_reg.register_phase(prepare.clone()),
        Err(EcsError::DuplicatePhase("prepare".to_string()))
    );

    // Unknown phase dependency edge
    assert_eq!(
        phase_reg.add_phase_edge(&prepare, &PhaseId::new("missing")),
        Err(EcsError::PhaseNotFound("missing".to_string()))
    );

    phase_reg.add_phase_edge(&prepare, &simulate).unwrap();
    assert!(matches!(
        phase_reg.add_phase_edge(&prepare, &simulate),
        Err(EcsError::DuplicatePhaseEdge { .. })
    ));

    // 3. SystemRegistry & AccessDescriptor validation
    let mut sys_reg = SystemRegistry::new();

    // Invalid access descriptor: reading and writing the same component
    let mut invalid_access = AccessDescriptor::new();
    invalid_access.add_read(id_pos);
    invalid_access.add_write(id_pos);

    let fail_result = sys_reg.register(
        "InvalidSystem".to_string(),
        Box::new(FunctionSystem::new(|_| Ok(()))),
        invalid_access,
        vec![],
    );
    assert!(matches!(
        fail_result,
        Err(EcsError::InvalidAccessDescriptor(_, _))
    ));

    // Valid access descriptor
    let mut valid_access = AccessDescriptor::new();
    valid_access.add_read(id_pos);
    valid_access.add_write(id_vel);

    let sys_id = sys_reg
        .register(
            "ValidSystem".to_string(),
            Box::new(FunctionSystem::new(|_| Ok(()))),
            valid_access,
            vec![],
        )
        .unwrap();

    assert_eq!(sys_reg.revision(), 1);
    assert!(sys_reg.get(sys_id).is_some());
}
