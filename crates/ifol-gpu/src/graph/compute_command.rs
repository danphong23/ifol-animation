use crate::resources::handle::{BindGroupHandle, BufferHandle, ComputePipelineHandle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeCommand {
    pub pipeline: ComputePipelineHandle,
    pub bind_groups: Vec<(u32, BindGroupHandle, Vec<u32>)>,
    pub workgroups: [u32; 3],
    pub indirect: Option<(BufferHandle, u64)>,
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
