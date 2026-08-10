# ifol-gpu Benchmark & Test Index

Tài liệu này liệt kê toàn bộ các kịch bản kiểm thử (Benchmarks & Visual Tests) được thiết kế để đảm bảo hiệu năng và tính chính xác của `ifol-gpu`. Hệ thống test được chia làm hai mảng:
- **`benches/render_benchmarks.rs`**: Chạy không giao diện (Headless CLI), ép GPU chạy cường độ cao nhất để đo lường giới hạn thời gian (microseconds).
- **`examples/`**: Mở cửa sổ đồ họa (Graphic Window) để kiểm chứng bằng mắt (Visual Verification). Đảm bảo GPU không chỉ chạy nhanh mà còn render ĐÚNG màu sắc/vị trí.

## Danh sách Kịch bản (Test Cases)

### 1. `bench_clear_screen`
- **Mục tiêu**: Đo lường **Overhead** (Độ trễ) gốc của toàn bộ hệ thống (Compiler + WGPU + Driver).
- **Thiết lập**: Đồ thị RenderGraph chỉ chứa 1 Node với RenderTarget, hoàn toàn không có lệnh vẽ (`DrawCommand`).
- **Kết quả mong đợi**: Hoàn thành trong khoảng `< 200 µs`. Chứng minh kiến trúc "mù lòa" không gây nghẽn CPU.

### 2. `bench_single_large_image` (Băng thông VRAM)
- **Mục tiêu**: Đo lường băng thông bộ nhớ của GPU khi xử lý Texture khổng lồ.
- **Thiết lập**: Nạp ảnh AI 4K (`assets/ai_demo_large.png`). RenderGraph có 1 lệnh vẽ (Quad phủ toàn màn hình).
- **Kết quả mong đợi**: GPU đọc và copy ảnh nặng từ VRAM ra Target mà không bị sụt giảm quá nhiều ms. Chứng minh hệ thống Texture Bind Group hoạt động chuẩn.

### 3. `bench_100k_sprites_cpu_stress` (Giới hạn của Compiler)
- **Mục tiêu**: Cố tình "hành hạ" phần mềm RenderGraphExecutor (Rust).
- **Thiết lập**: Nhồi **100,000** lệnh `DrawCommand` (mỗi lệnh vẽ 1 instance) vào 1 `RenderNode` duy nhất.
- **Kết quả mong đợi**: Đo lường vòng lặp `for cmd in commands` tốn bao nhiêu thời gian để dịch sang mã WGPU. Chỉ ra giới hạn của việc gửi từng lệnh vẽ riêng rẽ.

### 4. `bench_100k_sprites_gpu_instanced` (Sức mạnh phần cứng)
- **Mục tiêu**: Tối ưu hóa Case 3 bằng kỹ thuật **Instanced Rendering**.
- **Thiết lập**: Chỉ dùng **1** lệnh `DrawCommand` nhưng đặt `instance_count = 100_000`. Cung cấp dữ liệu tọa độ qua Instanced Buffer.
- **Kết quả mong đợi**: Tốc độ sẽ nhanh gấp hàng trăm lần Case 3, vì CPU được rảnh rỗi và GPU trực tiếp xử lý mảng dữ liệu. Đây là tiêu chuẩn để render Game 2D hiệu năng cao.

### 5. `bench_z_buffer_opaque` (Sắp ra mắt)
- **Mục tiêu**: Test Z-Buffer (Depth Test). Vẽ đè hàng vạn ảnh để đo tốc độ Culling. Không dùng kênh Alpha.

### 6. `bench_alpha_blending` (Sắp ra mắt)
- **Mục tiêu**: Đo lường chi phí hòa trộn bộ nhớ (Blend State) khi vẽ ảnh trong suốt.

### 7. `bench_pipeline_caching` (Sắp ra mắt)
- **Mục tiêu**: Test cơ chế Cache của WGPU bằng cách tái tạo lại RenderGraph phức tạp 2 lần liên tiếp.

---
*Ghi chú: Tài liệu này được tự động cập nhật bởi AI Agent mỗi khi hoàn thành tính năng mới.*
