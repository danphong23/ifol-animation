use super::{RawTextureReadback, ReadbackError};

pub struct ReadbackTicket {
    pub(super) buffer: wgpu::Buffer,
    pub(super) receiver: std::sync::mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
    pub(super) submission: wgpu::SubmissionIndex,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) bytes_per_pixel: u32,
    pub(super) padded_bytes_per_row: u32,
    pub(super) format: wgpu::TextureFormat,
}

impl ReadbackTicket {
    pub fn resolve_checked(
        self,
        device: &wgpu::Device,
    ) -> Result<RawTextureReadback, ReadbackError> {
        let _ = device.poll(wgpu::PollType::Wait {
            submission_index: Some(self.submission),
            timeout: None,
        });
        match self.receiver.recv() {
            Ok(Ok(())) => {}
            _ => return Err(ReadbackError::MapFailed),
        }
        let data = self
            .buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|_| ReadbackError::AccessFailed)?;
        let row_bytes = self
            .width
            .checked_mul(self.bytes_per_pixel)
            .ok_or(ReadbackError::ArithmeticOverflow)?;
        let capacity = row_bytes
            .checked_mul(self.height)
            .ok_or(ReadbackError::ArithmeticOverflow)? as usize;
        let mut pixels = Vec::with_capacity(capacity);
        for row in 0..self.height {
            let start = row
                .checked_mul(self.padded_bytes_per_row)
                .ok_or(ReadbackError::ArithmeticOverflow)? as usize;
            let end = start
                .checked_add(row_bytes as usize)
                .ok_or(ReadbackError::ArithmeticOverflow)?;
            let Some(row_data) = data.get(start..end) else {
                return Err(ReadbackError::AccessFailed);
            };
            pixels.extend_from_slice(row_data);
        }
        drop(data);
        self.buffer.unmap();
        Ok(RawTextureReadback {
            bytes: pixels,
            width: self.width,
            height: self.height,
            format: self.format,
        })
    }
}
