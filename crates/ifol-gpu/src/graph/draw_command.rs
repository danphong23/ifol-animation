use std::ops::Range;

use crate::resources::handle::{BindGroupHandle, BufferHandle, MeshHandle, PipelineHandle};

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
