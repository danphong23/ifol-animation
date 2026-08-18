use ifol_ecs::error::EcsError;
use ifol_ecs::query::Without;
use ifol_ecs::registry::{ComponentRegistry, PhaseId};
use ifol_ecs::system::{AccessDescriptor, Commands, RunCondition};
use ifol_ecs::{EcsRuntime, EntityId, World};

#[derive(Debug, Clone, Copy)]
struct Marker;

#[derive(Debug, Clone, Copy)]
struct Position;

#[test]
fn component_ids_with_the_same_local_index_are_not_interchangeable() {
    let mut foreign = ComponentRegistry::new();
    let foreign_id = foreign.register::<Position>().unwrap();

    let mut runtime = EcsRuntime::new();
    let local_id = runtime.register_component::<Position>().unwrap();
    assert_eq!(foreign_id.index(), local_id.index());

    let phase = PhaseId::new("validate.provenance");
    runtime.register_phase(phase.clone()).unwrap();
    let mut access = AccessDescriptor::new();
    access.add_read(foreign_id);
    let system = runtime
        .register_function_system("foreign", |_| Ok(()), access, vec![])
        .unwrap();
    runtime.attach_system(&phase, system).unwrap();

    assert!(matches!(
        runtime.compile(),
        Err(EcsError::ComponentIdNotRegistered(_))
    ));
}

#[test]
fn system_ids_with_the_same_local_index_are_not_interchangeable() {
    let mut source = EcsRuntime::new();
    let source_id = source
        .register_function_system("source", |_| Ok(()), AccessDescriptor::new(), vec![])
        .unwrap();

    let mut target = EcsRuntime::new();
    let target_id = target
        .register_function_system("target", |_| Ok(()), AccessDescriptor::new(), vec![])
        .unwrap();
    assert_ne!(format!("{source_id:?}"), format!("{target_id:?}"));
    let phase = PhaseId::new("validate.system-provenance");
    target.register_phase(phase.clone()).unwrap();

    assert!(matches!(
        target.attach_system(&phase, source_id),
        Err(EcsError::SystemNotFound(_))
    ));
}

#[test]
fn spawn_tickets_cannot_cross_command_buffers() {
    let mut world = World::new();
    let mut source = Commands::new();
    let foreign_ticket = source.spawn();
    let mut target = Commands::new();
    target.insert(foreign_ticket, Position);

    assert!(matches!(
        target.apply(&mut world),
        Err(EcsError::UnresolvedCommandTarget(_))
    ));
}

#[test]
fn failed_recompile_cannot_leave_an_old_schedule_executable() {
    let mut foreign = ComponentRegistry::new();
    let foreign_id = foreign.register::<Position>().unwrap();
    let mut runtime = EcsRuntime::new();
    let phase = PhaseId::new("compile.transaction");
    runtime.register_phase(phase.clone()).unwrap();
    runtime
        .register_function_system("ok", |_| Ok(()), AccessDescriptor::new(), vec![])
        .and_then(|system| runtime.attach_system(&phase, system))
        .unwrap();
    runtime.compile().unwrap();
    runtime.run_once().unwrap();

    let mut invalid_access = AccessDescriptor::new();
    invalid_access.add_read(foreign_id);
    runtime
        .register_function_system("invalid", |_| Ok(()), invalid_access, vec![])
        .unwrap();

    assert!(matches!(
        runtime.compile(),
        Err(EcsError::ComponentIdNotRegistered(_))
    ));
    assert!(matches!(
        runtime.run_once(),
        Err(EcsError::ScheduleNotCompiled)
    ));
}

#[test]
fn structural_commands_require_structural_access() {
    let mut runtime = EcsRuntime::new();
    let phase = PhaseId::new("structural.contract");
    runtime.register_phase(phase.clone()).unwrap();
    let entity = runtime.spawn();
    let denied = runtime
        .register_function_system(
            "denied",
            move |ctx| {
                ctx.commands().despawn(entity)?;
                Ok(())
            },
            AccessDescriptor::new(),
            vec![],
        )
        .unwrap();
    runtime.attach_system(&phase, denied).unwrap();
    runtime.compile().unwrap();
    let report = runtime.run_once().unwrap();
    assert_eq!(report.system_errors.len(), 1);
    assert!(runtime.world().is_alive(entity));

    let mut allowed_access = AccessDescriptor::new();
    allowed_access.add_structural();
    let allowed = runtime
        .register_function_system(
            "allowed",
            move |ctx| {
                ctx.commands().despawn(entity)?;
                Ok(())
            },
            allowed_access,
            vec![],
        )
        .unwrap();
    runtime.attach_system(&phase, allowed).unwrap();
    runtime.compile().unwrap();
    runtime.run_once().unwrap();
    assert!(!runtime.world().is_alive(entity));
}

#[test]
fn empty_any_condition_is_false() {
    let condition = RunCondition::Any(Vec::new());
    assert!(condition.evaluate(&World::new()).is_err());
}

#[test]
fn duplicate_system_names_are_rejected() {
    let mut runtime = EcsRuntime::new();
    runtime
        .register_function_system("same", |_| Ok(()), AccessDescriptor::new(), vec![])
        .unwrap();
    assert!(matches!(
        runtime.register_function_system("same", |_| Ok(()), AccessDescriptor::new(), vec![]),
        Err(EcsError::DuplicateSystem(_, _))
    ));
}

#[test]
fn large_modifier_query_does_not_recurse_per_rejected_entity() {
    let mut world = World::new();
    for _ in 0..20_000 {
        let entity = world.spawn();
        world.insert(entity, Marker).unwrap();
    }

    let mut query = world.query_mut::<Without<Marker>>().unwrap();
    assert_eq!(query.iter_with_entity().count(), 1);
}

#[test]
fn world_entity_remains_the_only_default_root_candidate() {
    let mut world = World::new();
    assert_eq!(world.query::<&'static Marker>().count(), 0);
    assert!(world.is_alive(EntityId::WORLD));
    world.insert_world_component(Marker);
    let marker_id = world.component_id::<Marker>().unwrap();
    assert!(
        world
            .component_registry()
            .descriptor(marker_id)
            .unwrap()
            .is_world_singleton
    );
}
