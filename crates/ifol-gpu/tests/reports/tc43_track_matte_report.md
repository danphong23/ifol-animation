# Báo cáo: TC43 - Track matte hai lớp

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/shared_assets/manifests/tc43_track_matte.json`
- **Graph fingerprint (FNV-1a):** `3824afc9c439d9b6`
- **Mô tả test case:** Dùng alpha silhouette của paladin làm mặt nạ để hiển thị nền Sci-Fi.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `track_matte.wgsl`
- **Asset/input:** `canonical_bg_scifi.png`, `canonical_sprites_heroes.png`
- **Chính sách input:** Desktop và WebGPU dùng hai PNG canonical: canonical_sprites_heroes.png và canonical_bg_scifi.png.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** matte_pass (không tên, target matte) → final_pass (không tên, target final)
- **Số pass:** `2`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `paladin_matte → alpha_matte`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `matte_pass → final_pass`
- **Graph resources:** nodes=`2`, draw commands=`2`, tổng instances=`2`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `4.9787 ms`
- **Thời gian render lần hai (warm/cache):** `1.1435 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `77.0%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `2 pass (chroma matte → track matte) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/outputs/desktop/tc43_track_matte_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `bd4412db6ead8b4b`
- **SHA-256:** `9dba5e9f0a618f086759503632166f4368a7782a114def098e00c469e96f2667`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc43_track_matte.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Sci-Fi chỉ xuất hiện trong silhouette paladin theo alpha matte; hai ảnh trùng nội dung.
- **Graph thực tế:** nodes=2, draw commands=2, instances=2



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `4.9000 ms`
- **Thời gian render lần hai (warm/cache):** `3.2000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `34.7%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `2 pass (chroma matte → track matte) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/outputs/web/tc43_track_matte_web.bin`
- **Dấu vân tay raw (FNV-1a):** `bd4412db6ead8b4b`
- **SHA-256:** `9dba5e9f0a618f086759503632166f4368a7782a114def098e00c469e96f2667`
- **Ảnh:** ![WebGPU output](../outputs/web/tc43_track_matte_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Sci-Fi chỉ xuất hiện trong silhouette paladin theo alpha matte; hai ảnh trùng nội dung.
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
