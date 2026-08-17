# Báo cáo: TC32 - Chuyển cảnh lật trang 3D

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc32_page_curl.json`
- **Graph fingerprint (FNV-1a):** `26faa4396e406466`
- **Mô tả test case:** Render hai cảnh khác nhau rồi chuyển từ cảnh paladin sang cảnh mage bằng biến dạng lật trang hình trụ.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `page_curl.wgsl`, `sky_composite_deterministic.wgsl`
- **Asset/input:** `canonical_sprites_heroes.png`, `canonical_tc085_noise.png`
- **Chính sách input:** Desktop và WebGPU dùng các fixture PNG canonical cho sprite sheet và noise; không dùng decoder JPEG trong phép đo parity.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** scene_a_pass (Paladin scene, target scene_a) → scene_b_pass (Mage scene, target scene_b) → curl_pass (Cylindrical page curl, target final)
- **Số pass:** `3`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `scene_a_sky → scene_a_paladin → scene_b_sky → scene_b_mage → page_curl_transition`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `scene_a_pass → scene_b_pass → curl_pass`
- **Graph resources:** nodes=`3`, draw commands=`5`, tổng instances=`5`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `6.5782 ms`
- **Thời gian render lần hai (warm/cache):** `2.4710 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `62.4%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `3 pass (scene A + scene B + page curl) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/desktop/tc32_page_curl_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `2d66910d012d2349`
- **SHA-256:** `88796fcd44a0e5b6606b2dff52b65b978d25f478df5269792d870af97ac3ae84`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc32_page_curl.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận ảnh cuối là chuyển cảnh page-curl 3D ở khoảng 50%: scene mage xanh ở bên trái, scene paladin tím ở bên phải, có dải cuộn ở giữa và bóng/biến dạng hình trụ; không có ảnh đen hoặc validation error.
- **Graph thực tế:** nodes=3, draw commands=5, instances=5



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `14.0000 ms`
- **Thời gian render lần hai (warm/cache):** `3.3000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `76.4%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `3 pass (scene A + scene B + page curl) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/web/tc32_page_curl_web.bin`
- **Dấu vân tay raw (FNV-1a):** `d647d43d96f34b5e`
- **SHA-256:** `df23129ffad868adad1b780cfe8caee5af1b08e187ed95ac6257c516ee630f44`
- **Ảnh:** ![WebGPU output](../outputs/web/tc32_page_curl_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận ảnh cuối là chuyển cảnh page-curl 3D ở khoảng 50%: scene mage xanh ở bên trái, scene paladin tím ở bên phải, có dải cuộn ở giữa và bóng/biến dạng hình trụ; không có ảnh đen hoặc validation error.
- **Graph thực tế:** nodes=3, draw commands=5, instances=5



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `3` |
| Số pixel khác nhau | `3` |
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
