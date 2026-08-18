# Báo cáo: TC48 - Độ sâu trường ảnh Bokeh

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/shared_assets/manifests/tc48_bokeh_dof.json`
- **Graph fingerprint (FNV-1a):** `6fefabed9d0cf1d5`
- **Mô tả test case:** Render scene paladin rồi làm mờ ngoài vùng focus bằng đĩa Bokeh lấy mẫu Fermat.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `bokeh_dof.wgsl`, `chroma_key_cropped.wgsl`, `texture_blit.wgsl`
- **Asset/input:** `canonical_bg_scifi.png`, `canonical_sprites_heroes.png`
- **Chính sách input:** Desktop và WebGPU dùng hai PNG canonical: canonical_sprites_heroes.png và canonical_bg_scifi.png.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** scene_pass (không tên, target scene) → bokeh_pass (không tên, target final)
- **Số pass:** `2`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `scene_background → scene_paladin → bokeh_dof`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `scene_pass → bokeh_pass`
- **Graph resources:** nodes=`2`, draw commands=`3`, tổng instances=`3`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `5.0652 ms`
- **Thời gian render lần hai (warm/cache):** `2.5644 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `49.4%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `2 pass scene/effect + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/outputs/desktop/tc48_bokeh_dof_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `e1765904afbc493b`
- **SHA-256:** `f7e564cc060e1c1632fedfb4877ae0d62acfe1d753cd488dbebd4bb2e6c443a0`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc48_bokeh_dof.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Foreground rõ hơn vùng nền, bokeh blur quanh vùng focus và highlight được boost; đúng mô tả.
- **Graph thực tế:** nodes=2, draw commands=3, instances=3



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `7.2000 ms`
- **Thời gian render lần hai (warm/cache):** `3.7000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `48.6%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `2 pass scene/effect (scene → bokeh depth of field) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/outputs/web/tc48_bokeh_dof_web.bin`
- **Dấu vân tay raw (FNV-1a):** `6ce329b0af3001e0`
- **SHA-256:** `686fdb80807412272f90a857685ef7a3efbf8e1e648bcea1fa05a24724f4d9a2`
- **Ảnh:** ![WebGPU output](../outputs/web/tc48_bokeh_dof_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Foreground rõ hơn vùng nền, bokeh blur quanh vùng focus và highlight được boost; đúng mô tả.
- **Graph thực tế:** nodes=2, draw commands=3, instances=3



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `71` |
| Số pixel khác nhau | `71` |
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
