pub use super::descriptors::*;
pub use super::ownership::OwnedTextureResource;
use super::versions::{ResourceVersion, ResourceVersions};
use crate::resources::handle::{
    BindGroupHandle, BufferHandle, ComputePipelineHandle, MeshHandle, PipelineHandle, TextureHandle,
};
use std::collections::HashMap;

/// Nơi ánh xạ từ Handle siêu nhẹ (u64) sang các đối tượng nặng của GPU (Buffer, Texture, Pipeline)
#[derive(Default)]
pub struct ResourceRegistry {
    pub(super) textures: HashMap<TextureHandle, (wgpu::TextureView, wgpu::TextureFormat)>,
    pub(super) pipelines: HashMap<PipelineHandle, wgpu::RenderPipeline>,
    pub(super) compute_pipelines: HashMap<ComputePipelineHandle, wgpu::ComputePipeline>,
    pub(super) pipeline_layout_descriptors: HashMap<PipelineHandle, PipelineLayoutResourceDescriptor>,
    pub(super) compute_pipeline_layout_descriptors:
        HashMap<ComputePipelineHandle, PipelineLayoutResourceDescriptor>,
    pub(super) buffers: HashMap<BufferHandle, wgpu::Buffer>,
    /// Lưu trữ Mesh: (VBO, Option<(IBO, IndexFormat)>, Số lượng Index/Vertex mặc định)
    pub(super) meshes: HashMap<MeshHandle, (wgpu::Buffer, Option<(wgpu::Buffer, wgpu::IndexFormat)>, u32)>,
    pub(super) mesh_descriptors: HashMap<MeshHandle, MeshResourceDescriptor>,
    pub(super) bind_groups: HashMap<BindGroupHandle, wgpu::BindGroup>,
    pub(super) bind_group_descriptors: HashMap<BindGroupHandle, BindGroupResourceDescriptor>,
    pub(super) buffer_descriptors: HashMap<BufferHandle, BufferResourceDescriptor>,
    pub(super) texture_descriptors: HashMap<TextureHandle, TextureResourceDescriptor>,
    pub(super) owned_textures: HashMap<TextureHandle, OwnedTextureResource>,
    pub(super) versions: ResourceVersions,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn texture_version(&self, handle: &TextureHandle) -> ResourceVersion {
        self.versions.textures.get(handle).copied().unwrap_or(0)
    }

    pub fn mark_texture_changed(&mut self, handle: TextureHandle) {
        self.bump_texture_version(handle);
    }

    pub fn pipeline_version(&self, handle: &PipelineHandle) -> ResourceVersion {
        self.versions.pipelines.get(handle).copied().unwrap_or(0)
    }

    pub fn mark_pipeline_changed(&mut self, handle: PipelineHandle) {
        self.bump_pipeline_version(handle);
    }

    pub fn compute_pipeline_version(&self, handle: &ComputePipelineHandle) -> ResourceVersion {
        self.versions
            .compute_pipelines
            .get(handle)
            .copied()
            .unwrap_or(0)
    }

    pub fn mark_compute_pipeline_changed(&mut self, handle: ComputePipelineHandle) {
        Self::bump_version(&mut self.versions.compute_pipelines, handle);
    }

    pub fn buffer_version(&self, handle: &BufferHandle) -> ResourceVersion {
        self.versions.buffers.get(handle).copied().unwrap_or(0)
    }

    pub fn mark_buffer_changed(&mut self, handle: BufferHandle) {
        Self::bump_version(&mut self.versions.buffers, handle);
    }

    pub fn mesh_version(&self, handle: &MeshHandle) -> ResourceVersion {
        self.versions.meshes.get(handle).copied().unwrap_or(0)
    }

    pub fn mark_mesh_changed(&mut self, handle: MeshHandle) {
        Self::bump_version(&mut self.versions.meshes, handle);
    }

    pub fn bind_group_version(&self, handle: &BindGroupHandle) -> ResourceVersion {
        self.versions.bind_groups.get(handle).copied().unwrap_or(0)
    }

    pub fn mark_bind_group_changed(&mut self, handle: BindGroupHandle) {
        Self::bump_version(&mut self.versions.bind_groups, handle);
    }

    pub(super) fn bump_texture_version(&mut self, handle: TextureHandle) {
        Self::bump_version(&mut self.versions.textures, handle);
    }

    pub(super) fn bump_pipeline_version(&mut self, handle: PipelineHandle) {
        Self::bump_version(&mut self.versions.pipelines, handle);
    }

    pub(super) fn bump_version<H: Copy + Eq + std::hash::Hash>(
        versions: &mut HashMap<H, ResourceVersion>,
        handle: H,
    ) {
        let version = versions.entry(handle).or_insert(0);
        *version = version.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::GpuEngineBuilder;
    use crate::memory::{DeferredDestructionQueue, SubmissionTracker};

    #[test]
    fn texture_version_starts_at_zero_and_marks_changes() {
        let mut registry = ResourceRegistry::new();
        let handle = TextureHandle(11);

        assert_eq!(registry.texture_version(&handle), 0);
        registry.mark_texture_changed(handle);
        assert_eq!(registry.texture_version(&handle), 1);
        registry.mark_texture_changed(handle);
        assert_eq!(registry.texture_version(&handle), 2);
    }

    #[test]
    fn versions_are_typed_and_independent() {
        let mut registry = ResourceRegistry::new();
        registry.mark_texture_changed(TextureHandle(1));
        registry.mark_pipeline_changed(PipelineHandle(1));

        assert_eq!(registry.texture_version(&TextureHandle(1)), 1);
        assert_eq!(registry.pipeline_version(&PipelineHandle(1)), 1);
        assert_eq!(registry.texture_version(&TextureHandle(2)), 0);
        assert_eq!(registry.pipeline_version(&PipelineHandle(2)), 0);
    }

    #[test]
    fn compute_pipeline_versions_are_independent_from_render_pipelines() {
        let mut registry = ResourceRegistry::new();
        registry.mark_pipeline_changed(PipelineHandle(1));
        registry.mark_compute_pipeline_changed(ComputePipelineHandle(1));

        assert_eq!(registry.pipeline_version(&PipelineHandle(1)), 1);
        assert_eq!(
            registry.compute_pipeline_version(&ComputePipelineHandle(1)),
            1
        );
    }

    #[test]
    fn buffer_versions_are_independent_from_texture_versions() {
        let mut registry = ResourceRegistry::new();
        registry.mark_buffer_changed(BufferHandle(1));
        registry.mark_texture_changed(TextureHandle(1));

        assert_eq!(registry.buffer_version(&BufferHandle(1)), 1);
        assert_eq!(registry.texture_version(&TextureHandle(1)), 1);
    }

    #[test]
    fn buffer_descriptor_rejects_invalid_size_and_usage() {
        assert_eq!(
            BufferResourceDescriptor {
                size: 0,
                usage: wgpu::BufferUsages::COPY_SRC
            }
            .validate(),
            Err(BufferDescriptorError::InvalidSize)
        );
        assert_eq!(
            BufferResourceDescriptor {
                size: 4,
                usage: wgpu::BufferUsages::empty()
            }
            .validate(),
            Err(BufferDescriptorError::EmptyUsage)
        );
        assert_eq!(
            BufferResourceDescriptor {
                size: 4,
                usage: wgpu::BufferUsages::COPY_SRC
            }
            .validate(),
            Ok(())
        );
    }

    #[test]
    fn mesh_descriptor_rejects_inconsistent_metadata() {
        assert_eq!(
            MeshResourceDescriptor {
                vertex_buffer_size: 0,
                vertex_count: 3,
                index_buffer_size: None,
                index_format: None
            }
            .validate(),
            Err(MeshDescriptorError::InvalidVertexBufferSize)
        );
        assert_eq!(
            MeshResourceDescriptor {
                vertex_buffer_size: 4,
                vertex_count: 0,
                index_buffer_size: None,
                index_format: None
            }
            .validate(),
            Err(MeshDescriptorError::InvalidVertexCount)
        );
        assert_eq!(
            MeshResourceDescriptor {
                vertex_buffer_size: 4,
                vertex_count: 3,
                index_buffer_size: Some(0),
                index_format: Some(wgpu::IndexFormat::Uint16)
            }
            .validate(),
            Err(MeshDescriptorError::InvalidIndexBufferSize)
        );
        assert_eq!(
            MeshResourceDescriptor {
                vertex_buffer_size: 4,
                vertex_count: 3,
                index_buffer_size: None,
                index_format: Some(wgpu::IndexFormat::Uint16)
            }
            .validate(),
            Err(MeshDescriptorError::IndexFormatWithoutBuffer)
        );
        assert_eq!(
            MeshResourceDescriptor {
                vertex_buffer_size: 4,
                vertex_count: 3,
                index_buffer_size: Some(6),
                index_format: Some(wgpu::IndexFormat::Uint16)
            }
            .validate(),
            Ok(())
        );
    }

    #[test]
    fn bind_group_descriptor_validates_dynamic_offset_contract() {
        assert_eq!(
            BindGroupResourceDescriptor {
                dynamic_offset_count: 0,
                dynamic_offset_alignment: 0,
                layout_signature: 7
            }
            .validate(),
            Ok(())
        );
        assert_eq!(
            BindGroupResourceDescriptor {
                dynamic_offset_count: 0,
                dynamic_offset_alignment: 256,
                layout_signature: 7
            }
            .validate(),
            Err(BindGroupDescriptorError::UnexpectedAlignmentWithoutOffsets)
        );
        assert_eq!(
            BindGroupResourceDescriptor {
                dynamic_offset_count: 1,
                dynamic_offset_alignment: 0,
                layout_signature: 7
            }
            .validate(),
            Err(BindGroupDescriptorError::InvalidAlignment)
        );
        assert_eq!(
            BindGroupResourceDescriptor {
                dynamic_offset_count: 2,
                dynamic_offset_alignment: 256,
                layout_signature: 7
            }
            .validate(),
            Ok(())
        );
    }

    fn valid_descriptor() -> TextureResourceDescriptor {
        TextureResourceDescriptor {
            width: 128,
            height: 64,
            depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            mip_level_count: 1,
            sample_count: 1,
        }
    }

    #[test]
    fn texture_descriptor_accepts_valid_input() {
        assert_eq!(valid_descriptor().validate(1024), Ok(()));
    }

    #[test]
    fn texture_descriptor_rejects_invalid_extent_and_limit() {
        let mut descriptor = valid_descriptor();
        descriptor.width = 0;
        assert_eq!(
            descriptor.validate(1024),
            Err(ResourceDescriptorError::InvalidExtent {
                width: 0,
                height: 64
            })
        );

        descriptor = valid_descriptor();
        descriptor.width = 2048;
        assert_eq!(
            descriptor.validate(1024),
            Err(ResourceDescriptorError::ExceedsDimensionLimit {
                width: 2048,
                height: 64,
                max_dimension: 1024
            })
        );
    }

    #[test]
    fn texture_descriptor_rejects_missing_shape_and_usage_fields() {
        let mut descriptor = valid_descriptor();
        descriptor.mip_level_count = 0;
        assert_eq!(
            descriptor.validate(1024),
            Err(ResourceDescriptorError::InvalidMipCount)
        );
        descriptor = valid_descriptor();
        descriptor.usage = wgpu::TextureUsages::empty();
        assert_eq!(
            descriptor.validate(1024),
            Err(ResourceDescriptorError::EmptyUsage)
        );
    }

    #[test]
    fn texture_descriptor_rejects_impossible_mips_and_sample_count() {
        let mut descriptor = valid_descriptor();
        descriptor.width = 8;
        descriptor.height = 4;
        descriptor.mip_level_count = 5;
        assert_eq!(
            descriptor.validate(1024),
            Err(ResourceDescriptorError::MipCountExceedsExtent {
                mip_level_count: 5,
                max_mip_level_count: 4,
                width: 8,
                height: 4,
            })
        );

        descriptor = valid_descriptor();
        descriptor.sample_count = 3;
        assert_eq!(
            descriptor.validate(1024),
            Err(ResourceDescriptorError::InvalidSampleCountValue { sample_count: 3 })
        );
    }

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
}
