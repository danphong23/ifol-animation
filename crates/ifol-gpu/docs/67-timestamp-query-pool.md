# IFOL GPU: timestamp query pool

## Primitive hiện tại

`TimestampQueryPool` là primitive profiling tùy chọn của core:

1. kiểm tra `TIMESTAMP_QUERY` khi tạo pool;
2. cấp từng `TimestampSpan` gồm hai query (begin/end);
3. ghi hai timestamp vào command encoder nếu backend có
   `TIMESTAMP_QUERY_INSIDE_ENCODERS`;
4. resolve span vào buffer do host sở hữu.

Pool không submit queue, không map buffer, không tự chờ submission và không tự
đoán backend. Host có thể dùng buffer resolve cùng `ReadbackTicket` hoặc pipeline
đọc GPU riêng.

## Invariant

- query count phải là số chẵn và ít nhất 2;
- span chỉ được cấp một lần từ pool và không vượt pool;
- resolve offset phải đúng alignment của `wgpu`;
- thiếu timestamp capability trả lỗi typed, không panic;
- thiếu encoder-timestamp capability chỉ ảnh hưởng `write_span`, không làm
  render thông thường thất bại.

## Giới hạn và bước tích hợp

Primitive này chưa tự chèn timestamp vào mọi pass của `RenderGraphExecutor`.
Việc đó cần policy chọn boundary, query lifetime theo frame và kết hợp với
`FrameContext`; sẽ được thực hiện như task riêng để không biến profiling tùy chọn
thành overhead bắt buộc.

