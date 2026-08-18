# Báo cáo: TC45 - Panel kính mờ glassmorphism

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/shared_assets/manifests/tc45_glassmorphism.json`
- **Graph fingerprint (FNV-1a):** `2d3810a87e84d9ac`
- **Mô tả test case:** Render scene Sci-Fi có paladin rồi phủ panel kính mờ bo góc, refraction và rim light.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `glassmorphism.wgsl`, `texture_blit.wgsl`
- **Asset/input:** `canonical_bg_scifi.png`, `canonical_sprites_heroes.png`
- **Chính sách input:** Desktop và WebGPU dùng hai PNG canonical: canonical_sprites_heroes.png và canonical_bg_scifi.png.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** scene_pass (không tên, target scene) → glass_pass (không tên, target final)
- **Số pass:** `2`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `scene_background → scene_paladin → glass_panel`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `scene_pass → glass_pass`
- **Graph resources:** nodes=`2`, draw commands=`3`, tổng instances=`3`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `4.5834 ms`
- **Thời gian render lần hai (warm/cache):** `1.6348 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `64.3%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `2 pass scene/effect + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/outputs/desktop/tc45_glassmorphism_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `0f1fe9f4d98f20a6`
- **SHA-256:** `8d35b604e25b4d599aa3b65886e65c6ffd2203dc73635e9639ae944a40d40874`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc45_glassmorphism.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Scene có paladin và panel kính mờ bo góc, blur/refraction/rim rõ; hai ảnh trùng.
- **Graph thực tế:** nodes=2, draw commands=3, instances=3



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `4.7000 ms`
- **Thời gian render lần hai (warm/cache):** `2.8000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `40.4%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `2 pass scene/effect (scene → glass panel) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/outputs/web/tc45_glassmorphism_web.bin`
- **Dấu vân tay raw (FNV-1a):** `631c61e5d0bf3caf`
- **SHA-256:** `1f9df9f4e7a8daac1852f20f05026d5f0e6dbe73eeb7e8b0cc212a1bb9afdf59`
- **Ảnh:** ![WebGPU output](../outputs/web/tc45_glassmorphism_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Scene có paladin và panel kính mờ bo góc, blur/refraction/rim rõ; hai ảnh trùng.
- **Graph thực tế:** nodes=2, draw commands=3, instances=3



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
