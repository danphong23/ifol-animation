use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureResourceDescriptor {
    pub width: u32,
    pub height: u32,
    pub depth_or_array_layers: u32,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
    pub mip_level_count: u32,
    pub sample_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferResourceDescriptor {
    pub size: u64,
    pub usage: wgpu::BufferUsages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshResourceDescriptor {
    pub vertex_buffer_size: u64,
    pub vertex_count: u32,
    pub index_buffer_size: Option<u64>,
    pub index_format: Option<wgpu::IndexFormat>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MeshDescriptorError {
    #[error("mesh vertex buffer size must be non-zero")]
    InvalidVertexBufferSize,
    #[error("mesh vertex count must be non-zero")]
    InvalidVertexCount,
    #[error("mesh index buffer size must be non-zero when present")]
    InvalidIndexBufferSize,
    #[error("mesh index format requires an index buffer")]
    IndexFormatWithoutBuffer,
}

impl MeshResourceDescriptor {
    pub fn validate(&self) -> Result<(), MeshDescriptorError> {
        if self.vertex_buffer_size == 0 {
            return Err(MeshDescriptorError::InvalidVertexBufferSize);
        }
        if self.vertex_count == 0 {
            return Err(MeshDescriptorError::InvalidVertexCount);
        }
        if self.index_buffer_size == Some(0) {
            return Err(MeshDescriptorError::InvalidIndexBufferSize);
        }
        if self.index_format.is_some() && self.index_buffer_size.is_none() {
            return Err(MeshDescriptorError::IndexFormatWithoutBuffer);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindGroupResourceDescriptor {
    pub dynamic_offset_count: u32,
    pub dynamic_offset_alignment: u32,
    pub layout_signature: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineLayoutResourceDescriptor {
    pub bind_group_layout_signatures: Vec<Option<u64>>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BindGroupDescriptorError {
    #[error("dynamic offset alignment must be zero when there are no dynamic offsets")]
    UnexpectedAlignmentWithoutOffsets,
    #[error("dynamic offset alignment must be a non-zero power of two")]
    InvalidAlignment,
}

impl BindGroupResourceDescriptor {
    pub fn validate(&self) -> Result<(), BindGroupDescriptorError> {
        if self.dynamic_offset_count == 0 {
            return if self.dynamic_offset_alignment == 0 {
                Ok(())
            } else {
                Err(BindGroupDescriptorError::UnexpectedAlignmentWithoutOffsets)
            };
        }
        if self.dynamic_offset_alignment == 0 || !self.dynamic_offset_alignment.is_power_of_two() {
            return Err(BindGroupDescriptorError::InvalidAlignment);
        }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BufferDescriptorError {
    #[error("buffer size must be non-zero")]
    InvalidSize,
    #[error("buffer usage must not be empty")]
    EmptyUsage,
}

impl BufferResourceDescriptor {
    pub fn validate(&self) -> Result<(), BufferDescriptorError> {
        if self.size == 0 {
            return Err(BufferDescriptorError::InvalidSize);
        }
        if self.usage.is_empty() {
            return Err(BufferDescriptorError::EmptyUsage);
        }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ResourceDescriptorError {
    #[error("texture width and height must be non-zero, got {width}x{height}")]
    InvalidExtent { width: u32, height: u32 },
    #[error("texture layer count must be non-zero")]
    InvalidLayerCount,
    #[error("texture mip level count must be non-zero")]
    InvalidMipCount,
    #[error("texture mip level count {mip_level_count} exceeds maximum {max_mip_level_count} for extent {width}x{height}")]
    MipCountExceedsExtent {
        mip_level_count: u32,
        max_mip_level_count: u32,
        width: u32,
        height: u32,
    },
    #[error("texture sample count must be non-zero")]
    InvalidSampleCount,
    #[error("texture sample count {sample_count} must be a power of two")]
    InvalidSampleCountValue { sample_count: u32 },
    #[error("texture usage must not be empty")]
    EmptyUsage,
    #[error("texture extent {width}x{height} exceeds device limit {max_dimension}")]
    ExceedsDimensionLimit {
        width: u32,
        height: u32,
        max_dimension: u32,
    },
}

impl TextureResourceDescriptor {
    pub fn validate(&self, max_dimension: u32) -> Result<(), ResourceDescriptorError> {
        if self.width == 0 || self.height == 0 {
            return Err(ResourceDescriptorError::InvalidExtent {
                width: self.width,
                height: self.height,
            });
        }
        if self.depth_or_array_layers == 0 {
            return Err(ResourceDescriptorError::InvalidLayerCount);
        }
        if self.mip_level_count == 0 {
            return Err(ResourceDescriptorError::InvalidMipCount);
        }
        let max_mip_level_count = u32::BITS - self.width.max(self.height).leading_zeros();
        if self.mip_level_count > max_mip_level_count {
            return Err(ResourceDescriptorError::MipCountExceedsExtent {
                mip_level_count: self.mip_level_count,
                max_mip_level_count,
                width: self.width,
                height: self.height,
            });
        }
        if self.sample_count == 0 {
            return Err(ResourceDescriptorError::InvalidSampleCount);
        }
        if !self.sample_count.is_power_of_two() {
            return Err(ResourceDescriptorError::InvalidSampleCountValue {
                sample_count: self.sample_count,
            });
        }
        if self.usage.is_empty() {
            return Err(ResourceDescriptorError::EmptyUsage);
        }
        if self.width > max_dimension || self.height > max_dimension {
            return Err(ResourceDescriptorError::ExceedsDimensionLimit {
                width: self.width,
                height: self.height,
                max_dimension,
            });
        }
        Ok(())
    }
}
