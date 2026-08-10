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
        // Căn lề block size theo yêu cầu của card đồ họa
        let aligned_size = (request_size + self.alignment - 1) & !(self.alignment - 1);
        
        if aligned_size > self.size {
            return None; // Kích thước yêu cầu vượt quá dung lượng tối đa của cả Ring
        }
        
        if self.current_offset + aligned_size <= self.size {
            let offset = self.current_offset;
            self.current_offset += aligned_size;
            Some(offset)
        } else {
            // Hết chỗ phía sau, vòng lại từ đầu Buffer
            self.current_offset = aligned_size;
            Some(0)
        }
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
        
        // Lần cấp phát tiếp theo sẽ bắt buộc Wrap-Around quay lại offset 0
        assert_eq!(ring.allocate(100), Some(0));
        assert_eq!(ring.current_offset, 256);
    }
}
