# Báo cáo: TC41 - Điền khung theo tỷ lệ và làm mờ nền

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/shared_assets/manifests/tc41_aspect_fill.json`
- **Graph fingerprint (FNV-1a):** `039c82c31366c5f1`
- **Mô tả test case:** Đưa nền Sci-Fi ngang vào target dọc 450x800, giữ vùng trung tâm và làm mờ phần nền điền khung.
- **Target:** `450x800`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `aspect_fill.wgsl`
- **Asset/input:** `canonical_bg_scifi.png`
- **Chính sách input:** Desktop và WebGPU dùng canonical_bg_scifi.png; không dùng decoder JPEG trong phép đo parity.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** fill_pass (Portrait aspect fill, target final)
- **Số pass:** `1`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `aspect_fill`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `fill_pass`
- **Graph resources:** nodes=`1`, draw commands=`1`, tổng instances=`1`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `4.5472 ms`
- **Thời gian render lần hai (warm/cache):** `2.6295 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `42.2%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `1 pass effect + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/outputs/desktop/tc41_aspect_fill_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `0be3a1d3db49f9a6`
- **SHA-256:** `9ba329707e8d1459e103497de23bc5e65b5db4370fe5cf7e39e34ab8e677f864`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc41_aspect_fill.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Ảnh Sci-Fi được fill khung 9:16, có nền blur bao quanh và foreground giữ tỷ lệ; Desktop/Web cùng bố cục.
- **Graph thực tế:** nodes=1, draw commands=1, instances=1



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `6.9000 ms`
- **Thời gian render lần hai (warm/cache):** `3.0000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `56.5%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `1 pass (aspect-fill background blur) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/outputs/web/tc41_aspect_fill_web.bin`
- **Dấu vân tay raw (FNV-1a):** `6384548d37a2e165`
- **SHA-256:** `de00ef3963e59ac71d521fe0433deb36b1ef1e2b09dae41afc10f5e7d3659389`
- **Ảnh:** ![WebGPU output](../outputs/web/tc41_aspect_fill_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Ảnh Sci-Fi được fill khung 9:16, có nền blur bao quanh và foreground giữ tỷ lệ; Desktop/Web cùng bố cục.
- **Graph thực tế:** nodes=1, draw commands=1, instances=1



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `13` |
| Số pixel khác nhau | `13` |
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
