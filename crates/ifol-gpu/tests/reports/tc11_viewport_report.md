# Báo cáo: TC11 - Hai viewport độc lập và compositor split-screen

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `crates/ifol-gpu/tests/shared_assets/manifests/tc11_viewport.json`
- **Graph fingerprint (FNV-1a):** `ba5edf11147d3095`
- **Mô tả test case:** Render hai viewport offscreen độc lập 400x600 rồi ghép thành target 800x600 bằng compositor và divider cố định.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `splitscreen_composite.wgsl`
- **Asset/input:** KHÔNG KHAI BÁO
- **Chính sách input:** Dùng asset theo manifest; chưa có chuẩn hóa input canonical riêng.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** left_viewport (Left isolated viewport, target left) → right_viewport (Right isolated viewport, target right) → final_composite (Deterministic split-screen composite, target final)
- **Số pass:** `3`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `split_composite`
- **Sampler contract:** `{"address_mode_u": "clamp-to-edge", "address_mode_v": "clamp-to-edge", "address_mode_w": "clamp-to-edge", "mag_filter": "nearest", "min_filter": "nearest", "mipmap_filter": "nearest"}`
- **Thứ tự layer kỳ vọng:** `left_viewport → right_viewport → final_composite`
- **Graph resources:** nodes=`3`, draw commands=`1`, tổng instances=`1`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `2.8198 ms`
- **Thời gian render lần hai (warm/cache):** `1.8382 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `34.8%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `3 pass execute_checked (left → right → final) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/desktop/tc11_viewport_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `ba90e6fc75081965`
- **SHA-256:** `5038577311cb296e0a262c88a75e1aab85eba21eb5b7501bbb189e6364afef85`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc11_viewport.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Hai viewport trái/phải có nền riêng, divider cyan-trắng nằm chính giữa, không có state leak hoặc artifact.
- **Graph thực tế:** nodes=3, draw commands=1, instances=None



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `3.8000 ms`
- **Thời gian render lần hai (warm/cache):** `3.8000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `0.0%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `3 pass offscreen (left → right → final) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/web/tc11_viewport_web.bin`
- **Dấu vân tay raw (FNV-1a):** `ba90e6fc75081965`
- **SHA-256:** `5038577311cb296e0a262c88a75e1aab85eba21eb5b7501bbb189e6364afef85`
- **Ảnh:** ![WebGPU output](../outputs/web/tc11_viewport_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Raw canonical image có cùng hai viewport trái/phải và divider cyan-trắng; không có state leak hoặc artifact.
- **Graph thực tế:** nodes=3, draw commands=1, instances=None



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
