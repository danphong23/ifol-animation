use std::ops::Range;

use crate::resources::handle::{
    BindGroupHandle, BufferHandle, ComputePipelineHandle, MeshHandle, PipelineHandle, TextureHandle,
};

use super::TextureAspect;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawAction {
    Indexed {
        mesh: MeshHandle,
        index_range: Range<u32>,
        instance_range: Range<u32>,
    },
    Procedural {
        vertex_count: u32,
        instance_range: Range<u32>,
    },
    Indirect {
        buffer: BufferHandle,
        offset: u64,
    },
    IndexedIndirect {
        mesh: MeshHandle,
        buffer: BufferHandle,
        offset: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawCommand {
    pub pipeline: PipelineHandle,
    pub bind_groups: Vec<(u32, BindGroupHandle, Vec<u32>)>,
    pub action: DrawAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeCommand {
    pub pipeline: ComputePipelineHandle,
    pub bind_groups: Vec<(u32, BindGroupHandle, Vec<u32>)>,
    pub workgroups: [u32; 3],
    pub indirect: Option<(BufferHandle, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyCommand {
    BufferToBuffer {
        source: BufferHandle,
        destination: BufferHandle,
        source_offset: u64,
        destination_offset: u64,
        size: u64,
    },
    TextureToTexture {
        source: TextureHandle,
        destination: TextureHandle,
        source_mip_level: u32,
        destination_mip_level: u32,
        source_origin: [u32; 3],
        destination_origin: [u32; 3],
        extent: [u32; 3],
    },
    TextureToTextureAspect {
        source: TextureHandle,
        destination: TextureHandle,
        source_mip_level: u32,
        destination_mip_level: u32,
        source_origin: [u32; 3],
        destination_origin: [u32; 3],
        extent: [u32; 3],
        aspect: TextureAspect,
    },
}

impl CopyCommand {
    pub fn buffer_to_buffer(source: BufferHandle, destination: BufferHandle, size: u64) -> Self {
        Self::BufferToBuffer {
            source,
            destination,
            source_offset: 0,
            destination_offset: 0,
            size,
        }
    }

    pub fn with_offsets(mut self, source_offset: u64, destination_offset: u64) -> Self {
        if let Self::BufferToBuffer {
            source_offset: source,
            destination_offset: destination,
            ..
        } = &mut self
        {
            *source = source_offset;
            *destination = destination_offset;
        }
        self
    }

    pub fn texture_to_texture(
        source: TextureHandle,
        destination: TextureHandle,
        extent: [u32; 3],
    ) -> Self {
        Self::TextureToTexture {
            source,
            destination,
            source_mip_level: 0,
            destination_mip_level: 0,
            source_origin: [0, 0, 0],
            destination_origin: [0, 0, 0],
            extent,
        }
    }

    pub fn texture_to_texture_aspect(
        source: TextureHandle,
        destination: TextureHandle,
        extent: [u32; 3],
        aspect: TextureAspect,
    ) -> Self {
        Self::TextureToTextureAspect {
            source,
            destination,
            source_mip_level: 0,
            destination_mip_level: 0,
            source_origin: [0, 0, 0],
            destination_origin: [0, 0, 0],
            extent,
            aspect,
        }
    }

    pub fn with_texture_mips(mut self, source_mip_level: u32, destination_mip_level: u32) -> Self {
        match &mut self {
            Self::TextureToTexture {
                source_mip_level: source,
                destination_mip_level: destination,
                ..
            }
            | Self::TextureToTextureAspect {
                source_mip_level: source,
                destination_mip_level: destination,
                ..
            } => {
                *source = source_mip_level;
                *destination = destination_mip_level;
            }
            _ => {}
        }
        self
    }
}

impl ComputeCommand {
    pub fn new(pipeline: ComputePipelineHandle, workgroups: [u32; 3]) -> Self {
        Self {
            pipeline,
            bind_groups: Vec::new(),
            workgroups,
            indirect: None,
        }
    }

    pub fn new_indirect(
        pipeline: ComputePipelineHandle,
        buffer: BufferHandle,
        offset: u64,
    ) -> Self {
        Self {
            pipeline,
            bind_groups: Vec::new(),
            workgroups: [0; 3],
            indirect: Some((buffer, offset)),
        }
    }

    pub fn with_bind_group(
        mut self,
        slot: u32,
        handle: BindGroupHandle,
        offsets: Vec<u32>,
    ) -> Self {
        self.bind_groups.push((slot, handle, offsets));
        self
    }
}

impl DrawCommand {
    pub fn new(pipeline: PipelineHandle, action: DrawAction) -> Self {
        Self {
            pipeline,
            bind_groups: Vec::new(),
            action,
        }
    }

    pub fn with_bind_group(
        mut self,
        slot: u32,
        handle: BindGroupHandle,
        offsets: Vec<u32>,
    ) -> Self {
        self.bind_groups.push((slot, handle, offsets));
        self
    }
}
