# Báo cáo: TC06 - Thu hồi node và tái sử dụng slot của RenderNodePool

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `crates/ifol-gpu/tests/shared_assets/manifests/tc06_gc.json`
- **Graph fingerprint (FNV-1a):** `adc851726f20d769`
- **Mô tả test case:** Cấp phát 100 render node, xóa 99 node đầu, giữ node cuối và chỉ render node còn sống đó.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`
- **Asset/input:** `canonical_sprites_warrior.png`
- **Chính sách input:** Dùng PNG canonical để Desktop/WebGPU giải mã cùng một input byte-level.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** Không khai báo dạng pass
- **Số pass:** `KHÔNG ÁP DỤNG`
- **Node pool contract:** `allocated=100, freed=99, surviving=1`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `5.3991 ms`
- **Thời gian render lần hai (warm/cache):** `1.1134 ms`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `execute_checked của graph còn một node + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/desktop/tc06_gc_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `1bc5893d3f44950e`
- **SHA-256:** `bba136d6d0164b1f291d73b0fd879e4e92fab8cd9be730da3a5ed10bfd43c702`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc06_gc.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: chỉ một warrior ở giữa ảnh; không có bản sao từ 99 node đã xóa; không thấy artifact.
- **Node pool thực tế:** allocated=100, freed=99, surviving=1

## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `41.7000 ms`
- **Thời gian render lần hai (warm/cache):** `2.0000 ms`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `execute offscreen của graph còn một node + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/web/tc06_gc_web.bin`
- **Dấu vân tay raw (FNV-1a):** `1bc5893d3f44950e`
- **SHA-256:** `bba136d6d0164b1f291d73b0fd879e4e92fab8cd9be730da3a5ed10bfd43c702`
- **Ảnh:** ![WebGPU output](../outputs/web/tc06_gc_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: chỉ một warrior ở giữa ảnh, bố cục giống Desktop; node pool invariant pass, không thấy artifact.
- **Node pool thực tế:** allocated=100, freed=99, surviving=1, check=True

## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `ĐẠT` |
| Số byte khác nhau | `0` |
| Số pixel khác nhau | `0` |
| Sai số kênh màu lớn nhất | `0/255` |
| Khác biệt màu/presentation | `KHÔNG` |
| Số pixel non-background Desktop/Web | `480000 / 480000` |
| Bounding box Desktop | `(0, 0, 799, 599)` |
| Bounding box WebGPU | `(0, 0, 799, 599)` |
| Bounding box non-background giống nhau | `ĐẠT` |
| Số pixel mask khác nhau | `0` (ngưỡng `0`) |
| Parity cấu trúc không phụ thuộc màu | `ĐẠT` |
| Đúng mô tả test case | `ĐẠT` |

**Kết luận:** `ĐẠT - output giống tuyệt đối từng byte.`

## 5. Phân tích hiệu suất

Các giá trị trên đo thời gian thực thi graph, submit lệnh và chờ GPU hoàn tất;
không bao gồm khởi tạo device/pipeline hoặc readback. Vì vậy `cold` ở đây là
lần execute đầu sau khi resource/pipeline đã được tạo, không phải cold start
của toàn bộ ứng dụng. Giá trị dưới `1 ms` tương đương microsecond và cần được
đọc theo đơn vị đó khi phân tích.
