use crate::resources::handle::{
    BindGroupHandle, BufferHandle, ComputePipelineHandle, MeshHandle, PipelineHandle, TextureHandle,
};
use std::collections::HashMap;

pub(crate) type ResourceVersion = u64;

#[derive(Default)]
pub(crate) struct ResourceVersions {
    pub(crate) textures: HashMap<TextureHandle, ResourceVersion>,
    pub(crate) pipelines: HashMap<PipelineHandle, ResourceVersion>,
    pub(crate) compute_pipelines: HashMap<ComputePipelineHandle, ResourceVersion>,
    pub(crate) buffers: HashMap<BufferHandle, ResourceVersion>,
    pub(crate) meshes: HashMap<MeshHandle, ResourceVersion>,
    pub(crate) bind_groups: HashMap<BindGroupHandle, ResourceVersion>,
}
