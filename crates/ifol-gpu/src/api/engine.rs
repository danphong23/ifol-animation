use std::sync::Arc;
use crate::api::capabilities::GpuCapabilities;

pub struct GpuEngine<'a> {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    capabilities: GpuCapabilities,
    surface: Option<wgpu::Surface<'a>>,
    surface_config: std::sync::RwLock<Option<wgpu::SurfaceConfiguration>>,
}

impl<'a> GpuEngine<'a> {
    pub(crate) fn new(
        device: wgpu::Device, 
        queue: wgpu::Queue, 
        capabilities: GpuCapabilities,
        surface: Option<wgpu::Surface<'a>>,
        surface_config: Option<wgpu::SurfaceConfiguration>,
    ) -> Self {
        Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            capabilities,
            surface,
            surface_config: std::sync::RwLock::new(surface_config),
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn capabilities(&self) -> &GpuCapabilities {
        &self.capabilities
    }

    pub fn surface(&self) -> Option<&wgpu::Surface<'a>> {
        self.surface.as_ref()
    }

    pub fn resize_surface(&self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            if let Some(surface) = &self.surface {
                let mut config_lock = self.surface_config.write().unwrap();
                if let Some(config) = config_lock.as_mut() {
                    config.width = width;
                    config.height = height;
                    surface.configure(&self.device, config);
                }
            }
        }
    }

    pub fn surface_format(&self) -> Option<wgpu::TextureFormat> {
        self.surface_config.read().unwrap().as_ref().map(|c| c.format)
    }

    /// Đọc toàn bộ byte của một Texture (2D) từ VRAM về CPU. Dùng để xuất file ảnh (PNG/JPEG) 
    /// phục vụ Automated Snapshot Testing hoặc kết xuất video (Offline Rendering).
    /// Hỗ trợ format `Rgba8UnormSrgb` (hoặc các format 4 byte/pixel tương tự).
    pub fn read_texture_to_bytes(&self, texture: &wgpu::Texture) -> Result<(Vec<u8>, u32, u32), &'static str> {
        let width = texture.size().width;
        let height = texture.size().height;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let bytes_per_pixel = 4;
        let unpadded_bytes = width * bytes_per_pixel;
        let padded_bytes = (unpadded_bytes + align - 1) & !(align - 1);
        
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ReadbackBuffer"),
            size: (padded_bytes * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo { 
                buffer: &buffer, 
                layout: wgpu::TexelCopyBufferLayout { 
                    offset: 0, 
                    bytes_per_row: Some(padded_bytes), 
                    rows_per_image: Some(height) 
                } 
            },
            texture.size()
        );
        let submission_index = self.queue.submit(std::iter::once(encoder.finish()));
        
        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |v| {
            let _ = tx.send(v);
        });
        
        // Cần poll device để WGPU thực thi lệnh copy và map buffer.
        let _ = self.device.poll(wgpu::PollType::Wait { submission_index: Some(submission_index), timeout: None });
        
        if rx.recv().is_err() {
            return Err("Failed to map buffer (receiver dropped)");
        }
        
        let data = slice.get_mapped_range().unwrap();
        let mut pixels = Vec::with_capacity((width * height * bytes_per_pixel) as usize);
        for row in 0..height {
            let start = (row * padded_bytes) as usize;
            pixels.extend_from_slice(&data[start..start + (width * bytes_per_pixel) as usize]);
        }
        
        Ok((pixels, width, height))
    }

    /// Đọc kết quả từ VRAM và lưu trực tiếp ra file ảnh trên ổ cứng.
    /// Bạn có thể truyền đường dẫn và tên file tùy ý. Đuôi file (.png, .jpg) sẽ tự định dạng loại ảnh.
    pub fn save_texture_to_file<P: AsRef<std::path::Path>>(&self, texture: &wgpu::Texture, path: P) -> Result<(), &'static str> {
        let (pixels, width, height) = self.read_texture_to_bytes(texture)?;
        
        // Đảm bảo thư mục tồn tại trước khi lưu
        if let Some(parent) = path.as_ref().parent() {
            if !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        
        if image::save_buffer(path, &pixels, width, height, image::ColorType::Rgba8).is_err() {
            return Err("Failed to save image to disk");
        }
        Ok(())
    }
}
