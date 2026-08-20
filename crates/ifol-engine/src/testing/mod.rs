//! Dev-only synthetic test packages for end-to-end multi-package pipeline validation.
//!
//! Models realistic feature packages without external crate dependencies:
//! - `pkg-timeline`: clock and time updates
//! - `pkg-motion`: entity physics and interpolation
//! - `pkg-renderer`: rasterization into mock framebuffers
//! - `pkg-audio`: sample mixing

use crate::package::PackageId;
use crate::registration::RegistrationContext;
use ifol_ecs::{AccessDescriptor, PhaseId, SystemContext};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

// ── Synthetic Components & Singletons ──────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClockTime {
    pub frame: u64,
    pub delta_seconds: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderBuffer {
    pub drawn_entities: Vec<u64>,
    pub frame_counter: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AudioSink {
    pub samples_rendered: usize,
}

// ── Package Definitions ───────────────────────────────────────────

pub struct TestTimelinePackage {
    pub phase: PhaseId,
}

impl TestTimelinePackage {
    pub fn new() -> Self {
        Self {
            phase: PhaseId::new("timeline"),
        }
    }

    pub fn id() -> PackageId {
        PackageId::new("pkg-test-timeline").unwrap()
    }

    pub fn register(&self, ctx: &mut RegistrationContext, frame_counter: Arc<AtomicU32>) {
        ctx.register_world_singleton::<ClockTime>();
        ctx.register_phase(self.phase.clone());

        ctx.register_system(
            "timeline_tick_system",
            self.phase.clone(),
            move |ctx: &mut SystemContext<'_>| {
                frame_counter.fetch_add(1, Ordering::SeqCst);
                let _ = ctx.system_name();
                Ok(())
            },
            AccessDescriptor::new(),
            vec![],
        );
    }
}

impl Default for TestTimelinePackage {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TestMotionPackage {
    pub phase: PhaseId,
}

impl TestMotionPackage {
    pub fn new() -> Self {
        Self {
            phase: PhaseId::new("motion"),
        }
    }

    pub fn id() -> PackageId {
        PackageId::new("pkg-test-motion").unwrap()
    }

    pub fn register(&self, ctx: &mut RegistrationContext, motion_counter: Arc<AtomicU32>) {
        ctx.register_phase(self.phase.clone());
        ctx.add_phase_edge(PhaseId::new("timeline"), self.phase.clone());

        ctx.register_system(
            "motion_integrate_system",
            self.phase.clone(),
            move |_ctx: &mut SystemContext<'_>| {
                motion_counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
            AccessDescriptor::new(),
            vec![],
        );
    }
}

impl Default for TestMotionPackage {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TestRendererPackage {
    pub phase: PhaseId,
}

impl TestRendererPackage {
    pub fn new() -> Self {
        Self {
            phase: PhaseId::new("render"),
        }
    }

    pub fn id() -> PackageId {
        PackageId::new("pkg-test-renderer").unwrap()
    }

    pub fn register(&self, ctx: &mut RegistrationContext, render_counter: Arc<AtomicU32>) {
        ctx.register_world_singleton::<RenderBuffer>();
        ctx.register_phase(self.phase.clone());
        ctx.add_phase_edge(PhaseId::new("motion"), self.phase.clone());

        ctx.register_system(
            "render_draw_system",
            self.phase.clone(),
            move |_ctx: &mut SystemContext<'_>| {
                render_counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
            AccessDescriptor::new(),
            vec![],
        );
    }
}

impl Default for TestRendererPackage {
    fn default() -> Self {
        Self::new()
    }
}
