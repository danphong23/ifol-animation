use thiserror::Error;

use crate::graph::TextureAspect;
use crate::resources::handle::{
    BindGroupHandle, BufferHandle, ComputePipelineHandle, MeshHandle, PipelineHandle, RenderNodeId,
    TextureHandle,
};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RenderGraphValidationError {
    #[error("extension operation {0:?} has no executor dispatch registered")]
    UnsupportedExtension(crate::extensions::ExtensionId),
    #[error("extension operation {extension:?} failed validation: {error}")]
    ExtensionValidation {
        extension: crate::extensions::ExtensionId,
        error: crate::extensions::ExtensionValidationError,
    },
    #[error("extension operation {extension:?} failed during dispatch: {error}")]
    ExtensionDispatch {
        extension: crate::extensions::ExtensionId,
        error: crate::extensions::ExtensionExecutionError,
    },
    #[error("render node {0:?} does not exist in the node pool")]
    MissingNode(RenderNodeId),
    #[error("render graph dependency cycle involves node {0:?}")]
    DependencyCycle(RenderNodeId),
    #[error("render graph dependency references node {0:?} outside the graph")]
    DependencyOutsideGraph(RenderNodeId),
    #[error("texture resource {0:?} is missing")]
    MissingTexture(TextureHandle),
    #[error("pipeline resource {0:?} is missing")]
    MissingPipeline(PipelineHandle),
    #[error("compute pipeline resource {0:?} is missing")]
    MissingComputePipeline(ComputePipelineHandle),
    #[error("buffer resource {0:?} is missing")]
    MissingBuffer(BufferHandle),
    #[error("declared resource usage references missing buffer {0:?}")]
    MissingUsageBuffer(BufferHandle),
    #[error("declared resource usage references missing texture {0:?}")]
    MissingUsageTexture(TextureHandle),
    #[error("buffer {handle:?} is missing required usage bits {required_usage:#x}; actual {actual_usage:#x}")]
    MissingBufferUsage {
        handle: BufferHandle,
        required_usage: u32,
        actual_usage: u32,
    },
    #[error("owned texture resource {0:?} is required for texture copy")]
    MissingOwnedTexture(TextureHandle),
    #[error("texture resource {0:?} has no descriptor metadata")]
    MissingTextureDescriptor(TextureHandle),
    #[error(
        "texture copy formats differ: source {source_handle:?}, destination {destination_handle:?}"
    )]
    TextureCopyFormatMismatch {
        source_handle: TextureHandle,
        destination_handle: TextureHandle,
    },
    #[error("texture copy extent must be non-zero, got {extent:?}")]
    InvalidTextureCopyExtent { extent: [u32; 3] },
    #[error("texture {handle:?} does not support copy aspect {aspect:?}")]
    InvalidTextureAspect {
        handle: TextureHandle,
        aspect: TextureAspect,
    },
    #[error(
        "texture copy mip level {mip_level} is invalid for {handle:?} (mip count {mip_count})"
    )]
    InvalidTextureMipLevel {
        handle: TextureHandle,
        mip_level: u32,
        mip_count: u32,
    },
    #[error("texture copy range for {handle:?} exceeds mip extent {mip_extent:?}: origin {origin:?}, extent {extent:?}")]
    InvalidTextureCopyRange {
        handle: TextureHandle,
        origin: [u32; 3],
        extent: [u32; 3],
        mip_extent: [u32; 3],
    },
    #[error("copy range for buffer {handle:?} exceeds buffer size: offset {offset}, size {size}, buffer size {buffer_size}")]
    InvalidCopyRange {
        handle: BufferHandle,
        offset: u64,
        size: u64,
        buffer_size: u64,
    },
    #[error("mesh resource {0:?} is missing")]
    MissingMesh(MeshHandle),
    #[error("bind group resource {0:?} is missing")]
    MissingBindGroup(BindGroupHandle),
    #[error("indirect buffer {0:?} is missing")]
    MissingIndirectBuffer(BufferHandle),
    #[error("indirect buffer {handle:?} is missing required usage bits {required_usage:#x}; actual {actual_usage:#x}")]
    MissingIndirectBufferUsage {
        handle: BufferHandle,
        required_usage: u32,
        actual_usage: u32,
    },
    #[error("indirect buffer {handle:?} range is invalid: offset {offset}, size {size}")]
    InvalidIndirectRange {
        handle: BufferHandle,
        offset: u64,
        size: u64,
    },
    #[error("indexed indirect draw requires mesh {0:?} to have an index buffer")]
    MissingIndexBuffer(MeshHandle),
    #[error("bind group slot {slot} is outside the device limit {max_slots}")]
    InvalidBindGroupSlot { slot: u32, max_slots: u32 },
    #[error("bind group {handle:?} expects {expected} dynamic offsets, got {actual}")]
    InvalidDynamicOffsetCount {
        handle: BindGroupHandle,
        expected: u32,
        actual: u32,
    },
    #[error("dynamic offset {offset} for bind group {handle:?} is not aligned to {alignment}")]
    InvalidDynamicOffsetAlignment {
        handle: BindGroupHandle,
        offset: u32,
        alignment: u32,
    },
    #[error("pipeline {pipeline:?} has no bind-group layout metadata for bind group {bind_group:?} at slot {slot}")]
    MissingPipelineLayoutMetadata {
        pipeline: PipelineHandle,
        bind_group: BindGroupHandle,
        slot: u32,
    },
    #[error("compute pipeline {pipeline:?} has no bind-group layout metadata for bind group {bind_group:?} at slot {slot}")]
    MissingComputePipelineLayoutMetadata {
        pipeline: ComputePipelineHandle,
        bind_group: BindGroupHandle,
        slot: u32,
    },
    #[error("pipeline {pipeline:?} layout mismatch at slot {slot}: expected {expected:?}, actual {actual:?}")]
    PipelineLayoutMismatch {
        pipeline: PipelineHandle,
        slot: u32,
        expected: Option<u64>,
        actual: Option<u64>,
    },
    #[error("compute pipeline {pipeline:?} layout mismatch at slot {slot}: expected {expected:?}, actual {actual:?}")]
    ComputePipelineLayoutMismatch {
        pipeline: ComputePipelineHandle,
        slot: u32,
        expected: Option<u64>,
        actual: Option<u64>,
    },
    #[error("render target dimensions must be non-zero, got {width}x{height}")]
    InvalidTargetSize { width: u32, height: u32 },
    #[error("texture {handle:?} has descriptor size {actual_width}x{actual_height}, graph requested {width}x{height}")]
    TargetSizeMismatch {
        handle: TextureHandle,
        width: u32,
        height: u32,
        actual_width: u32,
        actual_height: u32,
    },
    #[error("texture {handle:?} is missing required usage bits {required_usage:#x}; actual {actual_usage:#x}")]
    MissingTextureUsage {
        handle: TextureHandle,
        required_usage: u32,
        actual_usage: u32,
    },
    #[error("texture {handle:?} uses sample count {actual}, but this render path supports only sample count 1")]
    UnsupportedSampleCount { handle: TextureHandle, actual: u32 },
    #[error("MSAA resolve texture {0:?} is missing")]
    MissingResolveTexture(TextureHandle),
    #[error("MSAA resolve texture {handle:?} must be single-sample, got {actual}")]
    InvalidResolveSampleCount { handle: TextureHandle, actual: u32 },
    #[error("MSAA color and resolve formats differ: color {color:?}, resolve {resolve:?}")]
    ResolveFormatMismatch {
        color: wgpu::TextureFormat,
        resolve: wgpu::TextureFormat,
    },
    #[error("MSAA color and resolve dimensions differ: color {color_width}x{color_height}, resolve {resolve_width}x{resolve_height}")]
    ResolveSizeMismatch {
        color_width: u32,
        color_height: u32,
        resolve_width: u32,
        resolve_height: u32,
    },
    #[error("depth texture {handle:?} sample count mismatch: expected {expected}, got {actual}")]
    DepthSampleCountMismatch {
        handle: TextureHandle,
        expected: u32,
        actual: u32,
    },
}
