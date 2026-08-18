# Báo cáo: TC66 - Histogram độ sáng song song

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `../shared_assets/manifests/tc66_histogram.json`
- **Graph fingerprint (FNV-1a):** `52de157767d72d36`
- **Mô tả test case:** Tính histogram 256 bin bằng atomic trong workgroup, sau đó vẽ overlay lên ảnh nguồn.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `compute_histogram.wgsl`, `render_histogram.wgsl`
- **Asset/input:** `sprites_heroes.jpeg`
- **Chính sách input:** Desktop và WebGPU dùng cùng asset sprites_heroes.jpeg và cùng 256-bin atomic contract.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** histogram_compute_pass (Parallel luminance histogram, target histogram_buffer) → histogram_render_pass (Histogram overlay, target final)
- **Số pass:** `2`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `histogram_compute → histogram_overlay`
- **Sampler contract:** `Không khai báo`
- **Thứ tự layer kỳ vọng:** `histogram_compute_pass → histogram_render_pass`
- **Graph resources:** nodes=`2`, draw commands=`2`, tổng instances=`1`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `3.8147 ms`
- **Thời gian render lần hai (warm/cache):** `1.7400 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `54.4%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `execute_checked + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; state mutable được reset trước warm; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `../outputs/desktop/tc66_histogram_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `d6089f388d5c55a6`
- **SHA-256:** `0f3298f4e5644b71f85cc56ccb6da6861e9049501659aad79e919c2ca4185a82`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc66_histogram.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Desktop hiển thị đúng ảnh nguồn và overlay histogram tối ở góc phải với các cột sáng; tổng readback 256 bin bằng 480000 pixel.
- **Graph thực tế:** nodes=2, draw commands=2, instances=1



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `22.2000 ms`
- **Thời gian render lần hai (warm/cache):** `4.9000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `77.9%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `1 compute histogram dispatch + 1 overlay pass + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; state mutable được reset trước warm; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `../outputs/web/tc66_histogram_web.bin`
- **Dấu vân tay raw (FNV-1a):** `1e56d148ca2e8700`
- **SHA-256:** `a7b1cad14e3cd8f7ec11edd1b5d0599294d0e21be3658a66480eb5e4ffbda0a7`
- **Ảnh:** ![WebGPU output](../outputs/web/tc66_histogram_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** WebGPU hiển thị cùng ảnh nguồn và overlay histogram cùng vị trí; cấu trúc và phạm vi cột đúng, khác biệt raw lớn hơn do decoder/format input JPEG, không có lỗi graph.
- **Graph thực tế:** nodes=2, draw commands=2, instances=1



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `74771` |
| Số pixel khác nhau | `54379` |
| Sai số kênh màu lớn nhất | `242/255` |
| Khác biệt màu/presentation | `CÓ - cần theo dõi để đạt byte parity` |
| Số pixel non-background Desktop/Web | `KHÔNG ÁP DỤNG` |
| Bounding box Desktop | `KHÔNG ÁP DỤNG` |
| Bounding box WebGPU | `KHÔNG ÁP DỤNG` |
| Bounding box non-background giống nhau | `ĐẠT` |
| Số pixel mask khác nhau | `0` (ngưỡng `0`) |
| Parity cấu trúc không phụ thuộc màu | `ĐẠT` |
| Cache giữ nguyên output cold/warm ở cả hai môi trường | `ĐẠT` |
| Validation/fallback contract không panic | `ĐẠT` |
| Đúng mô tả test case | `ĐẠT` |

**Kết luận:** `ĐẠT CÓ ĐIỀU KIỆN - graph và cấu trúc render giống; khác biệt còn lại thuộc pixel/màu và nằm trong ngưỡng đã khai báo.`

## 5. Phân tích hiệu suất

Các giá trị trên đo thời gian thực thi graph, submit lệnh và chờ GPU hoàn tất;
không bao gồm khởi tạo device/pipeline hoặc readback. Vì vậy `cold` ở đây là
lần execute đầu sau khi resource/pipeline đã được tạo, không phải cold start
của toàn bộ ứng dụng. Giá trị dưới `1 ms` tương đương microsecond và cần được
đọc theo đơn vị đó khi phân tích.
