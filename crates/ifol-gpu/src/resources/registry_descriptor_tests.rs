use super::*;

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
