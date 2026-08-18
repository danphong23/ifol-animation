#![allow(dead_code)]

use ifol_ecs::error::SystemError;
use ifol_ecs::system::{AccessDescriptor, RunCondition, System, SystemContext};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// Test entity components
#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct Velocity {
    pub dx: f32,
    pub dy: f32,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Health(pub i32);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Name(pub String);

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct OptionalTag;

#[derive(Debug, Clone)]
pub struct DropTracker {
    pub counter: Arc<AtomicUsize>,
}

impl Drop for DropTracker {
    fn drop(&mut self) {
        self.counter.fetch_add(1, Ordering::SeqCst);
    }
}

// Test world singleton components on WORLD_ENTITY
#[derive(Debug, PartialEq, Clone)]
pub struct TestConfig {
    pub speed_multiplier: f32,
    pub title: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct RunCounter {
    pub ticks: u64,
}

#[derive(Debug, Clone)]
pub struct MockServiceHandle {
    pub service_id: u32,
}

// Standard test systems
pub struct MovementSystem;

impl System for MovementSystem {
    fn run(&mut self, ctx: &mut SystemContext<'_>) -> Result<(), SystemError> {
        let speed = ctx
            .world_ref::<TestConfig>()
            .map(|c| c.speed_multiplier)
            .unwrap_or(1.0);

        let query = ctx.query::<(&'static Position, &'static Velocity)>();
        let updates: Vec<(ifol_ecs::EntityId, Position)> = query
            .iter_with_entity()
            .map(|(e, (pos, vel))| {
                (
                    e,
                    Position {
                        x: pos.x + vel.dx * speed,
                        y: pos.y + vel.dy * speed,
                    },
                )
            })
            .collect();

        for (e, new_pos) in updates {
            if let Some(pos) = ctx.get_mut::<Position>(e) {
                *pos = new_pos;
            }
        }

        if let Some(counter) = ctx.world_mut::<RunCounter>() {
            counter.ticks += 1;
        }

        Ok(())
    }
}

pub struct FailingSystem;

impl System for FailingSystem {
    fn run(&mut self, _ctx: &mut SystemContext<'_>) -> Result<(), SystemError> {
        Err(SystemError::new("intentional test failure"))
    }
}

/// Helper function to build a standard movement system registration.
pub fn movement_system_reg() -> (
    &'static str,
    MovementSystem,
    AccessDescriptor,
    Vec<RunCondition>,
) {
    ("MovementSystem", MovementSystem, AccessDescriptor::new(), vec![])
}
