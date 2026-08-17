# Báo cáo: TC08.5 - Phân bổ ánh trăng định hướng và cảnh đêm hữu cơ

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `crates/ifol-gpu/tests/shared_assets/manifests/tc08_5_nightsky.json`
- **Graph fingerprint (FNV-1a):** `68208dc42cb1942b`
- **Mô tả test case:** Render sky procedural, 100 sao, mặt trăng, 4 lớp mây có silver lining và một pass post-bloom; graph dùng 2 pass scene → final.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `cloud_depth.wgsl`, `moon_surface.wgsl`, `postprocess_night_bloom.wgsl`, `sky_composite.wgsl`, `star_particles_sprite.wgsl`
- **Asset/input:** `canonical_tc085_noise.png`, `canonical_tc085_props.png`
- **Chính sách input:** Dùng PNG canonical để Desktop/WebGPU giải mã cùng một input byte-level.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** scene (Procedural sky and celestial layers, target scene) → final (Celestial bloom post-process, target final)
- **Số pass:** `2`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `sky → stars → moon → cloud_1 → cloud_2 → cloud_3 → cloud_4 → post`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "nearest", "min_filter": "nearest", "mipmap_filter": "nearest"}`
- **Thứ tự layer kỳ vọng:** `Không khai báo`
- **Graph resources:** nodes=`2`, draw commands=`8`, tổng instances=`107`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `3.7073 ms`
- **Thời gian render lần hai (warm/cache):** `1.8962 ms`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `execute_checked của 2 pass scene → final + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/desktop/tc08_5_nightsky_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `d4d017a04de34a15`
- **SHA-256:** `4c34883bf625ae6cf23bdace66f04c14faab9aad52e231851f38afa441dbfd11`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc08_5_nightsky.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Ảnh có nền trời đêm procedural, mặt trăng ở góc trên trái, sao, bốn lớp mây theo chiều sâu và bloom; không thấy artifact rõ ràng.
- **Graph thực tế:** nodes=2, draw commands=8, instances=None


## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `7.9000 ms`
- **Thời gian render lần hai (warm/cache):** `3.5000 ms`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `execute offscreen của 2 pass scene → final + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/web/tc08_5_nightsky_web.bin`
- **Dấu vân tay raw (FNV-1a):** `1a68c75bb91d493a`
- **SHA-256:** `8aacf44f41678770cd0eec86b7bcf89f076f4ffe8fb632454f1a17b688f34902`
- **Ảnh:** ![WebGPU output](../outputs/web/tc08_5_nightsky_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Ảnh có cùng nền trời đêm procedural, mặt trăng ở góc trên trái, sao, bốn lớp mây theo chiều sâu và bloom; không thấy artifact rõ ràng.
- **Graph thực tế:** nodes=2, draw commands=8, instances=None


## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `1` |
| Số pixel khác nhau | `1` |
| Sai số kênh màu lớn nhất | `1/255` |
| Khác biệt màu/presentation | `CÓ - cần theo dõi để đạt byte parity` |
| Số pixel non-background Desktop/Web | `KHÔNG ÁP DỤNG` |
| Bounding box Desktop | `KHÔNG ÁP DỤNG` |
| Bounding box WebGPU | `KHÔNG ÁP DỤNG` |
| Bounding box non-background giống nhau | `ĐẠT` |
| Số pixel mask khác nhau | `0` (ngưỡng `0`) |
| Parity cấu trúc không phụ thuộc màu | `ĐẠT` |
| Đúng mô tả test case | `ĐẠT` |

**Kết luận:** `ĐẠT CÓ ĐIỀU KIỆN - graph và cấu trúc render giống; khác biệt còn lại thuộc pixel/màu và nằm trong ngưỡng đã khai báo.`

## 5. Phân tích hiệu suất

Các giá trị trên đo thời gian thực thi graph, submit lệnh và chờ GPU hoàn tất;
không bao gồm khởi tạo device/pipeline hoặc readback. Vì vậy `cold` ở đây là
lần execute đầu sau khi resource/pipeline đã được tạo, không phải cold start
của toàn bộ ứng dụng. Giá trị dưới `1 ms` tương đương microsecond và cần được
đọc theo đơn vị đó khi phân tích.
