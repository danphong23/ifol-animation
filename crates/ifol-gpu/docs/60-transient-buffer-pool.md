# Transient buffer pool

`TransientBufferPool` quản lý handle buffer tạm theo:

- descriptor key chính xác (`size`, `usage`);
- `SubmissionId` cuối cùng sử dụng buffer;
- chỉ acquire khi submission đã completed;
- chống duplicate release;
- `drain_completed` để host giải phóng backing buffer.

Pool chỉ quản lý lifetime metadata/handle, không tự tạo hoặc xóa
`wgpu::Buffer`; host/resource registry chịu trách nhiệm backing object. Đây là
ranh giới tương tự `TransientTexturePool` và là nền cho frame context sau này.
