pub struct UniformRingBuffer {
    buffer: wgpu::Buffer,
    size: u64,
    current_offset: u64,
    alignment: u64,
}

impl UniformRingBuffer {
    pub fn new(device: &wgpu::Device, size: u64, alignment: u32) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UniformRingBuffer"),
            size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { 
            buffer, 
            size, 
            current_offset: 0, 
            alignment: alignment as u64 
        }
    }

    /// Cấp phát một khoảng trống trên Ring Buffer, trả về offset đầu tiên.
    /// Nếu không đủ chỗ trống phía sau, con trỏ sẽ quay vòng (wrap-around) về 0.
    pub fn allocate(&mut self, request_size: u64) -> Option<u64> {
        if request_size == 0 || self.alignment == 0 {
            return None;
        }

        let aligned_size = request_size
            .checked_add(self.alignment - 1)?
            / self.alignment
            * self.alignment;
        if aligned_size > self.size {
            return None;
        }

        // Không tự wrap: vùng đầu buffer có thể vẫn đang được GPU tham chiếu.
        let end = self.current_offset.checked_add(aligned_size)?;
        if end > self.size {
            return None;
        }

        let offset = self.current_offset;
        self.current_offset = end;
        Some(offset)
    }

    /// Ghi dữ liệu trực tiếp vào Ring Buffer qua wgpu::Queue và trả về Dynamic Offset
    pub fn write<T: bytemuck::Pod>(&mut self, queue: &wgpu::Queue, data: &T) -> Option<u64> {
        let size = std::mem::size_of::<T>() as u64;
        let offset = self.allocate(size)?;
        queue.write_buffer(&self.buffer, offset, bytemuck::bytes_of(data));
        Some(offset)
    }

    pub fn reset(&mut self) {
        self.current_offset = 0;
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::GpuEngineBuilder;

    #[test]
    fn test_ring_buffer_wrap_around() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        // Mock thông số căn lề chuẩn là 256 bytes để dễ test logic
        let alignment = 256;
        
        // Cố tình tạo buffer nhỏ (1024 bytes) để test wrap around dễ dàng
        let mut ring = UniformRingBuffer::new(engine.device(), 1024, alignment);
        
        // Cấp phát 100 bytes -> bị ép căn lề thành 256 bytes.
        assert_eq!(ring.allocate(100), Some(0));
        assert_eq!(ring.allocate(200), Some(256));
        assert_eq!(ring.allocate(500), Some(512)); 
        // 512 + 512 (căn lề) = 1024. Đã xài hết buffer.
        
        // Không được tự wrap và ghi đè allocation cũ.
        assert_eq!(ring.allocate(100), None);
        assert_eq!(ring.current_offset, 1024);

        ring.reset();
        assert_eq!(ring.allocate(100), Some(0));
    }

    #[test]
    fn ring_rejects_zero_and_overflowing_requests() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let mut ring = UniformRingBuffer::new(engine.device(), 1024, 256);

        assert_eq!(ring.allocate(0), None);
        assert_eq!(ring.allocate(u64::MAX), None);
    }
}
