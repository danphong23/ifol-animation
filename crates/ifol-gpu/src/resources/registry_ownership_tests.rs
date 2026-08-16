use super::*;
use crate::backend::GpuEngineBuilder;
use crate::memory::{DeferredDestructionQueue, SubmissionTracker};

#[test]
fn owned_texture_keeps_texture_object_and_descriptor_together() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("owned_texture_test"),
        size: wgpu::Extent3d {
            width: 16,
            height: 8,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let mut registry = ResourceRegistry::new();
    let descriptor = TextureResourceDescriptor {
        width: 16,
        height: 8,
        depth_or_array_layers: 1,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        mip_level_count: 1,
        sample_count: 1,
    };

    registry
        .insert_owned_texture(TextureHandle(3), texture, descriptor, 1024)
        .unwrap();
    assert!(registry.owned_texture(&TextureHandle(3)).is_some());
    assert_eq!(
        registry.texture_descriptor(&TextureHandle(3)),
        Some(&descriptor)
    );
    assert!(registry.texture(&TextureHandle(3)).is_some());
    assert!(registry.remove_owned_texture(&TextureHandle(3)).is_some());
    assert!(registry.owned_texture(&TextureHandle(3)).is_none());
    assert!(registry.texture(&TextureHandle(3)).is_none());
}

#[test]
fn owned_texture_deferred_removal_waits_for_submission_completion() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("deferred_owned_texture_test"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let descriptor = TextureResourceDescriptor {
        width: 4,
        height: 4,
        depth_or_array_layers: 1,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
        mip_level_count: 1,
        sample_count: 1,
    };
    let mut registry = ResourceRegistry::new();
    registry
        .insert_owned_texture(TextureHandle(9), texture, descriptor, 1024)
        .unwrap();
    let mut tracker = SubmissionTracker::new();
    let last_use = tracker.begin();
    let mut queue = DeferredDestructionQueue::new();
    assert!(registry.defer_owned_texture_destruction(&TextureHandle(9), last_use, &mut queue));
    assert!(registry.owned_texture(&TextureHandle(9)).is_none());
    assert_eq!(queue.pending_count(), 1);
    assert!(queue.drain_completed(&tracker).is_empty());
    tracker.mark_completed(last_use);
    assert_eq!(queue.drain_completed(&tracker).len(), 1);
    assert!(!registry.defer_owned_texture_destruction(&TextureHandle(9), last_use, &mut queue));
}
