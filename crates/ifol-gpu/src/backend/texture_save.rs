use super::engine::GpuEngine;
use super::readback::ReadbackError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TextureSaveError {
    #[error(transparent)]
    Readback(#[from] ReadbackError),
    #[error("could not create parent directory {path:?}: {source}")]
    CreateDirectory {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("image encoding failed: {0}")]
    Encode(#[from] image::ImageError),
}

impl<'a> GpuEngine<'a> {
    pub fn save_texture_to_file_checked<P: AsRef<std::path::Path>>(
        &self,
        texture: &wgpu::Texture,
        path: P,
    ) -> Result<(), TextureSaveError> {
        let readback = self.read_texture_to_raw_with_format_checked(
            texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )?;
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|source| {
                    TextureSaveError::CreateDirectory {
                        path: parent.to_path_buf(),
                        source,
                    }
                })?;
            }
        }
        image::save_buffer(
            path,
            &readback.bytes,
            readback.width,
            readback.height,
            image::ColorType::Rgba8,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::TextureSaveError;

    #[test]
    fn checked_texture_save_reports_encode_failure() {
        let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("checked-save-error-test"),
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
        let parent =
            std::env::temp_dir().join(format!("ifol_gpu_save_parent_{}", std::process::id()));
        std::fs::write(&parent, b"file, not directory").unwrap();
        let result = engine.save_texture_to_file_checked(&texture, parent.join("output.png"));
        let _ = std::fs::remove_file(&parent);
        assert!(matches!(result, Err(TextureSaveError::Encode(_))));
    }
}
