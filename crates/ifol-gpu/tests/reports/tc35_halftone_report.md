# Báo cáo: TC35 - Bộ lọc in lưới điểm

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `crates/ifol-gpu/tests/shared_assets/manifests/tc35_halftone.json`
- **Graph fingerprint (FNV-1a):** `0bfdc815933931d8`
- **Mô tả test case:** Tách nhân vật paladin canonical rồi chuyển thành các chấm halftone xoay 45 độ theo độ sáng.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `halftone.wgsl`
- **Asset/input:** `canonical_sprites_heroes.png`
- **Chính sách input:** Desktop và WebGPU dùng sprite sheet PNG canonical; không dùng decoder JPEG trong phép đo parity.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** chroma_pass (Chroma key extraction, target chroma) → halftone_pass (45 degree halftone, target final)
- **Số pass:** `2`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `chroma_extract_paladin → halftone_45deg`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `chroma_pass → halftone_pass`
- **Graph resources:** nodes=`2`, draw commands=`2`, tổng instances=`2`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `3.9052 ms`
- **Thời gian render lần hai (warm/cache):** `1.0198 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `73.9%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `2 pass (chroma key → 45 degree halftone) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/desktop/tc35_halftone_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `0605110999aa29fa`
- **SHA-256:** `94b1071ec3e295eb0fbf208c1b73f55331b5cf1a5b71616249b6bc333f9b607c`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc35_halftone.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận nền vàng comic có nhân vật paladin được chuyển thành các chấm halftone đen/trắng theo lưới xoay 45 độ; hình dáng vẫn nhận diện được, không có ảnh đen bất thường hoặc validation error.
- **Graph thực tế:** nodes=2, draw commands=2, instances=2



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `20.9000 ms`
- **Thời gian render lần hai (warm/cache):** `3.5000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `83.3%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `2 pass (chroma key → 45 degree halftone) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/web/tc35_halftone_web.bin`
- **Dấu vân tay raw (FNV-1a):** `84cc280769bb1d60`
- **SHA-256:** `5c01e4447a1811a2b00e558e2e72503755c24d11cc60820db1af0d7d074cbabb`
- **Ảnh:** ![WebGPU output](../outputs/web/tc35_halftone_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận kết quả Web có cùng bố cục, nền vàng comic và nhân vật halftone như Desktop; không có ảnh đen bất thường hoặc validation error.
- **Graph thực tế:** nodes=2, draw commands=2, instances=2



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `6` |
| Số pixel khác nhau | `2` |
| Sai số kênh màu lớn nhất | `1/255` |
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
