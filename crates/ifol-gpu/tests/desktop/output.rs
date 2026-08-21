use ifol_gpu::backend::GpuEngine;
use std::path::Path;

pub fn save_texture_as_png(
    engine: &GpuEngine<'_>,
    texture: &wgpu::Texture,
    format: wgpu::TextureFormat,
    path: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let readback = engine.read_texture_to_raw_with_format_checked(texture, format)?;
    let mut bytes = readback.bytes;

    match format {
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => {}
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
            for pixel in bytes.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }
        other => {
            return Err(format!("PNG helper only supports 8-bit RGBA/BGRA, got {other:?}").into());
        }
    }

    let path = path.as_ref();
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    image::save_buffer(
        path,
        &bytes,
        readback.width,
        readback.height,
        image::ColorType::Rgba8,
    )?;
    Ok(())
}
