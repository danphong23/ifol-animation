mod support;

use ifol_ecs::{AccessDescriptor, Commands, EcsError, EcsRuntime, EntityId, World};
use support::{Health, Position};

#[test]
fn commands_resolve_spawn_tickets_in_order() {
    let mut world = World::new();
    let mut commands = Commands::new();

    let first = commands.spawn();
    commands.insert(first, Position { x: 1.0, y: 0.0 });
    let second = commands.spawn();
    commands.insert(second, Position { x: 2.0, y: 0.0 });

    assert_eq!(commands.apply(&mut world).unwrap(), 4);
    assert_eq!(world.query::<&'static Position>().count(), 2);
}

#[test]
fn command_errors_are_returned_and_remaining_actions_are_not_replayed() {
    let mut world = World::new();
    let dead = world.spawn();
    world.despawn(dead).unwrap();
    let _replacement = world.spawn();

    let mut commands = Commands::new();
    commands.insert(dead, Position { x: 1.0, y: 0.0 });
    let _unapplied = commands.spawn();

    assert_eq!(
        commands.apply(&mut world),
        Err(EcsError::EntityNotFound(dead))
    );
    assert!(commands.is_empty());
    assert_eq!(world.entity_count(), 2);
}

#[test]
fn system_commands_enforce_write_access_and_discard_on_system_failure() {
    let mut runtime = EcsRuntime::new();
    runtime.register_component::<Health>().unwrap();
    let phase = ifol_ecs::PhaseId::new("commands.contract");
    runtime.register_phase(phase.clone()).unwrap();

    let denied = runtime
        .register_function_system(
            "DeniedCommand",
            |ctx| {
                ctx.commands().insert(EntityId::WORLD, Health(1))?;
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
    assert!(runtime.get_world_component::<Health>().is_none());
}

#[test]
fn system_failure_clears_already_queued_commands() {
    let mut runtime = EcsRuntime::new();
    runtime.register_component::<Health>().unwrap();
    let phase = ifol_ecs::PhaseId::new("commands.failure");
    runtime.register_phase(phase.clone()).unwrap();

    let mut access = AccessDescriptor::new();
    access.add_write(runtime.world().component_id::<Health>().unwrap());
    let system = runtime
        .register_function_system(
            "FailAfterQueue",
            |ctx| {
                ctx.commands().insert(EntityId::WORLD, Health(2))?;
                Err(ifol_ecs::SystemError::new("intentional failure"))
            },
            access,
            vec![],
        )
        .unwrap();
    runtime.attach_system(&phase, system).unwrap();
    runtime.compile().unwrap();

    let report = runtime.run_once().unwrap();
    assert_eq!(report.system_errors.len(), 1);
    assert_eq!(report.commands_processed, 0);
    assert!(runtime.get_world_component::<Health>().is_none());
}

#[test]
fn system_can_spawn_and_initialize_an_entity_with_a_ticket() {
    let mut runtime = EcsRuntime::new();
    runtime.register_component::<Position>().unwrap();
    let phase = ifol_ecs::PhaseId::new("commands.spawn");
    runtime.register_phase(phase.clone()).unwrap();

    let mut access = AccessDescriptor::new();
    access.add_write(runtime.world().component_id::<Position>().unwrap());
    let system = runtime
        .register_function_system(
            "SpawnAndInitialize",
            |ctx| {
                let ticket = ctx.commands().spawn();
                ctx.commands().insert(ticket, Position { x: 4.0, y: 5.0 })?;
                Ok(())
            },
            access,
            vec![],
        )
        .unwrap();
    runtime.attach_system(&phase, system).unwrap();
    runtime.compile().unwrap();
    runtime.run_once().unwrap();

    assert_eq!(runtime.query::<&'static Position>().count(), 1);
}
