use super::{texture_format_bytes_per_pixel, ReadbackError};

#[test]
fn readback_format_width_is_explicit() {
    assert_eq!(
        texture_format_bytes_per_pixel(wgpu::TextureFormat::R8Unorm),
        Some(1)
    );
    assert_eq!(
        texture_format_bytes_per_pixel(wgpu::TextureFormat::Rgba8UnormSrgb),
        Some(4)
    );
    assert_eq!(
        texture_format_bytes_per_pixel(wgpu::TextureFormat::Rgba16Float),
        Some(8)
    );
    assert_eq!(
        texture_format_bytes_per_pixel(wgpu::TextureFormat::Depth32Float),
        None
    );
}

#[test]
fn async_readback_ticket_resolves_after_submission() {
    let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
    let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("async-readback-test"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    engine.queue().write_texture(
        texture.as_image_copy(),
        &[1, 2, 3, 4],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        texture.size(),
    );

    let ticket = engine
        .begin_texture_readback_checked(&texture, wgpu::TextureFormat::Rgba8Unorm)
        .unwrap();
    let readback = ticket.resolve_checked(engine.device()).unwrap();
    assert_eq!((readback.width, readback.height), (1, 1));
    assert_eq!(readback.format, wgpu::TextureFormat::Rgba8Unorm);
    assert_eq!(readback.bytes, vec![1, 2, 3, 4]);
}

#[test]
fn checked_readback_rejects_unsupported_format_with_typed_error() {
    let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
    let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("checked-readback-format-test"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    assert!(matches!(
        engine.begin_texture_readback_checked(&texture, wgpu::TextureFormat::Depth32Float),
        Err(ReadbackError::UnsupportedFormat(
            wgpu::TextureFormat::Depth32Float
        ))
    ));
}

#[test]
fn registry_readback_uses_owned_texture_descriptor_format() {
    let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
    let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("registry-readback-test"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    engine.queue().write_texture(
        texture.as_image_copy(),
        &[9, 8, 7, 6],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        texture.size(),
    );

    let mut registry = crate::resources::ResourceRegistry::new();
    let handle = crate::resources::TextureHandle(11);
    registry
        .insert_owned_texture(
            handle,
            texture,
            crate::resources::TextureResourceDescriptor {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
                mip_level_count: 1,
                sample_count: 1,
            },
            1,
        )
        .unwrap();

    let readback = engine
        .read_texture_to_raw_from_registry_checked(&registry, &handle)
        .unwrap();
    assert_eq!(readback.format, wgpu::TextureFormat::Rgba8Unorm);
    assert_eq!(readback.bytes, vec![9, 8, 7, 6]);
}
