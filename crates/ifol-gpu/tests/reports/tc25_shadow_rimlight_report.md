# Báo cáo: TC25 - Rim light giả lập và đổ bóng

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc25_shadow_rimlight.json`
- **Graph fingerprint (FNV-1a):** `15cf62a1e76cb4e2`
- **Mô tả test case:** Render một instance bóng và một instance chính trong cùng draw command, thêm viền sáng ở biên sprite.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `rimlight.wgsl`
- **Asset/input:** `canonical_sprites_heroes.png`
- **Chính sách input:** Dùng PNG canonical để Desktop/WebGPU giải mã cùng một input byte-level.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** rimlight_scene (Shadow and rim light, target final)
- **Số pass:** `1`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `shadow_and_rimlight`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `rimlight_scene`
- **Graph resources:** nodes=`1`, draw commands=`1`, tổng instances=`2`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `4.4486 ms`
- **Thời gian render lần hai (warm/cache):** `0.9806 ms (980.6 µs)`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `78.0%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `1 pass (shadow instance + rimlight instance) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/desktop/tc25_shadow_rimlight_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `59fe86681428ea5f`
- **SHA-256:** `02cb9e8a2f1169c00b9c475c862f5f71e56f7767553fb04a07f6e22f831ef980`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc25_shadow_rimlight.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận sprite chính có viền sáng vàng ở biên và bóng đổ lệch phía sau; nhân vật vẫn rõ, alpha hợp lệ, không có black output hoặc artefact bất thường.
- **Graph thực tế:** nodes=1, draw commands=1, instances=2



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `17.9000 ms`
- **Thời gian render lần hai (warm/cache):** `3.6000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `79.9%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `1 pass (shadow instance + rimlight instance) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/web/tc25_shadow_rimlight_web.bin`
- **Dấu vân tay raw (FNV-1a):** `59fe86681428ea5f`
- **SHA-256:** `02cb9e8a2f1169c00b9c475c862f5f71e56f7767553fb04a07f6e22f831ef980`
- **Ảnh:** ![WebGPU output](../outputs/web/tc25_shadow_rimlight_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận output Web trùng Desktop; viền sáng vàng và bóng đổ lệch phía sau đúng mô tả, không có black output hoặc artefact bất thường.
- **Graph thực tế:** nodes=1, draw commands=1, instances=2



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
