# Báo cáo: TC36 - Làm nhòe tỏa tâm

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `crates/ifol-gpu/tests/shared_assets/manifests/tc36_radial_blur.json`
- **Graph fingerprint (FNV-1a):** `e8635023d0c9c2fb`
- **Mô tả test case:** Tách nhân vật paladin canonical rồi áp dụng blur tỏa tâm với 30 mẫu và trọng số giảm dần.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `radial_blur.wgsl`
- **Asset/input:** `canonical_sprites_heroes.png`
- **Chính sách input:** Desktop và WebGPU dùng sprite sheet PNG canonical; không dùng decoder JPEG trong phép đo parity.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** chroma_pass (Chroma key extraction, target chroma) → blur_pass (Radial zoom blur, target final)
- **Số pass:** `2`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `chroma_extract_paladin → radial_blur_30_samples`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `chroma_pass → blur_pass`
- **Graph resources:** nodes=`2`, draw commands=`2`, tổng instances=`2`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `4.1131 ms`
- **Thời gian render lần hai (warm/cache):** `1.7967 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `56.3%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `2 pass (chroma key → radial zoom blur) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/desktop/tc36_radial_blur_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `3bf21e7e4cfad443`
- **SHA-256:** `86cf1f33f8417b5cb951e5a5fa1fb7d100e110a1a61fc5552c9cff2f540539ec`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc36_radial_blur.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận nền tím tối có nhân vật paladin bị làm nhòe tỏa tâm từ giữa ảnh; nhân vật vẫn nhận diện được, hiệu ứng zoom blur rõ, không có ảnh đen bất thường hoặc validation error.
- **Graph thực tế:** nodes=2, draw commands=2, instances=2



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `34.3000 ms`
- **Thời gian render lần hai (warm/cache):** `3.4000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `90.1%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `2 pass (chroma key → radial zoom blur) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/web/tc36_radial_blur_web.bin`
- **Dấu vân tay raw (FNV-1a):** `3bf21e7e4cfad443`
- **SHA-256:** `86cf1f33f8417b5cb951e5a5fa1fb7d100e110a1a61fc5552c9cff2f540539ec`
- **Ảnh:** ![WebGPU output](../outputs/web/tc36_radial_blur_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận kết quả Web có cùng nền tím, bố cục và hiệu ứng radial blur như Desktop; nhân vật vẫn nhận diện được, không có ảnh đen bất thường hoặc validation error.
- **Graph thực tế:** nodes=2, draw commands=2, instances=2



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
| Số pixel non-background Desktop/Web | `KHÔNG ÁP DỤNG` |
| Bounding box Desktop | `KHÔNG ÁP DỤNG` |
| Bounding box WebGPU | `KHÔNG ÁP DỤNG` |
| Bounding box non-background giống nhau | `ĐẠT` |
| Số pixel mask khác nhau | `0` (ngưỡng `0`) |
| Parity cấu trúc không phụ thuộc màu | `ĐẠT` |
| Cache giữ nguyên output cold/warm ở cả hai môi trường | `ĐẠT` |
| Validation/fallback contract không panic | `ĐẠT` |
| Đúng mô tả test case | `ĐẠT` |

**Kết luận:** `ĐẠT - output giống tuyệt đối từng byte.`

## 5. Phân tích hiệu suất

Các giá trị trên đo thời gian thực thi graph, submit lệnh và chờ GPU hoàn tất;
không bao gồm khởi tạo device/pipeline hoặc readback. Vì vậy `cold` ở đây là
lần execute đầu sau khi resource/pipeline đã được tạo, không phải cold start
của toàn bộ ứng dụng. Giá trị dưới `1 ms` tương đương microsecond và cần được
đọc theo đơn vị đó khi phân tích.
