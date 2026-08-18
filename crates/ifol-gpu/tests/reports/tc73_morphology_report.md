# Báo cáo: TC73 - Dilation mask bằng compute trên GPU

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `crates/ifol-gpu/tests/shared_assets/manifests/tc73_morphology.json`
- **Graph fingerprint (FNV-1a):** `a7b1a9742b8aea01`
- **Mô tả test case:** Tạo mask gồm vòng tròn và các chấm rồi dilation radius 10 trên storage texture 800x800.
- **Target:** `800x800`, `Rgba8Unorm`
- **Shader/WGSL:** `compute_morphology.wgsl`
- **Asset/input:** KHÔNG KHAI BÁO
- **Chính sách input:** Desktop và WebGPU tạo cùng mask procedural trong WGSL; không dùng texture decoder hoặc asset bên ngoài.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** mask_pass (Procedural ring and dot mask, target mask_texture) → morphology_pass (Radius-10 dilation, target final)
- **Số pass:** `2`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `generate_mask → dilate_mask`
- **Sampler contract:** `Không khai báo`
- **Thứ tự layer kỳ vọng:** `mask_pass → morphology_pass`
- **Graph resources:** nodes=`1`, draw commands=`2`, tổng instances=`0`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `21.0161 ms`
- **Thời gian render lần hai (warm/cache):** `19.6383 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Warm diff chi tiết:** `bytes=0, pixels=0, max_delta=0/255, tolerance=0`
- **Speedup cold → warm:** `6.6%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `execute_checked + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; state mutable được reset hoặc ghi đè trước warm; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/desktop/tc73_morphology_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `ff5467dec4a391ec`
- **SHA-256:** `c0181b521a1d7860e094ac3dea27a40a76d513ffc8475994c2c0740b736dd4c3`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc73_morphology.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** TC73: Desktop hiển thị vòng trắng đã dilation cùng ba cụm chấm vuông trên nền đen; phù hợp radius 10.
 - **Numeric validation:** `{"height": 800, "mask_rewritten_before_warm": true, "mode": "dilation", "nonzero_pixel_count": 51159, "radius": 10, "width": 800}`
- **Graph thực tế:** nodes=1, draw commands=2, instances=0



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `32.5000 ms`
- **Thời gian render lần hai (warm/cache):** `18.0000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Warm diff chi tiết:** `bytes=0, pixels=0, max_delta=0/255, tolerance=0`
- **Speedup cold → warm:** `44.6%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `2 compute passes (mask generation + radius-10 dilation) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; state mutable được reset trước warm; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/web/tc73_morphology_web.bin`
- **Dấu vân tay raw (FNV-1a):** `ff5467dec4a391ec`
- **SHA-256:** `c0181b521a1d7860e094ac3dea27a40a76d513ffc8475994c2c0740b736dd4c3`
- **Ảnh:** ![WebGPU output](../outputs/web/tc73_morphology_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** TC73: Web hiển thị cùng hình học morphology trên nền đen; vòng và ba cụm chấm trùng với Desktop.
 - **Numeric validation:** `{"width": 800, "height": 800, "radius": 10, "mode": "dilation", "nonzero_pixel_count": 51159, "mask_rewritten_before_warm": true}`
- **Graph thực tế:** nodes=1, draw commands=2, instances=0



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
| Số pixel non-background Desktop/Web | `51159 / 51159` |
| Bounding box Desktop | `(135, 135, 666, 666)` |
| Bounding box WebGPU | `(135, 135, 666, 666)` |
| Bounding box non-background giống nhau | `ĐẠT` |
| Số pixel mask khác nhau | `0` (ngưỡng `5000`) |
| Parity cấu trúc không phụ thuộc màu | `ĐẠT` |
| Cache giữ nguyên output cold/warm ở cả hai môi trường | `ĐẠT` |
| Warm diff chi tiết đã được ghi nhận | `Desktop: bytes=0, pixels=0, max_delta=0/255, tolerance=0; WebGPU: bytes=0, pixels=0, max_delta=0/255, tolerance=0` |
| Validation/fallback contract không panic | `ĐẠT` |
| Đúng mô tả test case | `ĐẠT` |

**Kết luận:** `ĐẠT - output giống tuyệt đối từng byte.`

## 5. Phân tích hiệu suất

Các giá trị trên đo thời gian thực thi graph, submit lệnh và chờ GPU hoàn tất;
không bao gồm khởi tạo device/pipeline hoặc readback. Vì vậy `cold` ở đây là
lần execute đầu sau khi resource/pipeline đã được tạo, không phải cold start
của toàn bộ ứng dụng. Giá trị dưới `1 ms` tương đương microsecond và cần được
đọc theo đơn vị đó khi phân tích.
