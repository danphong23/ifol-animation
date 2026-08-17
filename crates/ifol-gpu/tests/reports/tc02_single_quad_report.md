# Báo cáo: TC02 - Sprite tứ giác đơn với Chroma Key

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc02_single_quad.json`
- **Graph fingerprint (FNV-1a):** `7b6d39865368457e`
- **Mô tả test case:** Render một sprite pháp sư đã crop ở giữa nền tối và loại bỏ phông xanh.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`
- **Asset/input:** `canonical_sprites_heroes.png`
- **Chính sách input:** Dùng PNG canonical để Desktop/WebGPU giải mã cùng một input byte-level.
- **Depth/stencil:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `6.4355 ms`
- **Thời gian render lần hai (warm/cache):** `0.8977 ms (897.7 µs)`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `execute_checked + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/desktop/tc02_single_quad_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `f90d1d187573bd44`
- **SHA-256:** `2d2058e865247b4258bd837d9fe725efe8b99642540a43df1f2f6a855b911539`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc02_single_quad.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: pháp sư nằm giữa ảnh, crop đúng, nền xóa sạch, không thấy viền xanh.

## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `9.1000 ms`
- **Thời gian render lần hai (warm/cache):** `3.4000 ms`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `execute offscreen + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/web/tc02_single_quad_web.bin`
- **Dấu vân tay raw (FNV-1a):** `00eb47cd67549da9`
- **SHA-256:** `049b021fe047c495e8647234a53ec83af2304e1d8b46d3b9f47a3099ddcc1bce`
- **Ảnh:** ![WebGPU output](../outputs/web/tc02_single_quad_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: bố cục, vị trí và nền sprite đúng mô tả; ảnh canonical từ raw readback.

## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `78440` |
| Số pixel khác nhau | `34224` |
| Sai số kênh màu lớn nhất | `77/255` |
| Khác biệt màu/presentation | `CÓ - cần theo dõi để đạt byte parity` |
| Số pixel non-background Desktop/Web | `54878 / 54707` |
| Bounding box Desktop | `(305, 65, 494, 535)` |
| Bounding box WebGPU | `(305, 65, 494, 535)` |
| Bounding box non-background giống nhau | `ĐẠT` |
| Số pixel mask khác nhau | `171` (ngưỡng `256`) |
| Parity cấu trúc không phụ thuộc màu | `ĐẠT` |
| Đúng mô tả test case | `ĐẠT` |

**Kết luận:** `ĐẠT CÓ ĐIỀU KIỆN - graph và cấu trúc render giống; khác biệt còn lại thuộc pixel/màu và nằm trong ngưỡng đã khai báo.`

## 5. Phân tích hiệu suất

Các giá trị trên đo thời gian thực thi graph, submit lệnh và chờ GPU hoàn tất;
không bao gồm khởi tạo device/pipeline hoặc readback. Vì vậy `cold` ở đây là
lần execute đầu sau khi resource/pipeline đã được tạo, không phải cold start
của toàn bộ ứng dụng. Giá trị dưới `1 ms` tương đương microsecond và cần được
đọc theo đơn vị đó khi phân tích.
