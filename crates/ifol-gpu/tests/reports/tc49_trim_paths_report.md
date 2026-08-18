# Báo cáo: TC49 - Trim Paths nét đứt neon

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/shared_assets/manifests/tc49_trim_paths.json`
- **Graph fingerprint (FNV-1a):** `49cdb72f893223c4`
- **Mô tả test case:** Render scene mage rồi phủ khung bo góc nét đứt cyan được cắt theo phần trăm chu vi.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `texture_blit.wgsl`, `trim_paths.wgsl`
- **Asset/input:** `canonical_bg_scifi.png`, `canonical_sprites_heroes.png`
- **Chính sách input:** Desktop và WebGPU dùng hai PNG canonical: canonical_sprites_heroes.png và canonical_bg_scifi.png.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** scene_pass (không tên, target scene) → trim_pass (không tên, target final)
- **Số pass:** `2`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `scene_background → scene_mage → trim_paths`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `scene_pass → trim_pass`
- **Graph resources:** nodes=`2`, draw commands=`3`, tổng instances=`3`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `3.6727 ms`
- **Thời gian render lần hai (warm/cache):** `1.2880 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `64.9%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `2 pass scene/effect + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/outputs/desktop/tc49_trim_paths_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `46dc733c8f9be4c7`
- **SHA-256:** `cbde554d4ab637337192dfb73adcf308f4c64de3d1d84fca42393640689433a0`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc49_trim_paths.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Mage trên nền với khung bo góc nét đứt cyan bị trim theo start/end; Desktop/Web trùng exact.
- **Graph thực tế:** nodes=2, draw commands=3, instances=3



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `6.3000 ms`
- **Thời gian render lần hai (warm/cache):** `2.9000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `54.0%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `2 pass scene/effect (scene → trimmed neon stroke) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/outputs/web/tc49_trim_paths_web.bin`
- **Dấu vân tay raw (FNV-1a):** `46dc733c8f9be4c7`
- **SHA-256:** `cbde554d4ab637337192dfb73adcf308f4c64de3d1d84fca42393640689433a0`
- **Ảnh:** ![WebGPU output](../outputs/web/tc49_trim_paths_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Mage trên nền với khung bo góc nét đứt cyan bị trim theo start/end; Desktop/Web trùng exact.
- **Graph thực tế:** nodes=2, draw commands=3, instances=3



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
