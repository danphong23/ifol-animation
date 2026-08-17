# Báo cáo: TC10 - Tài nguyên thiếu: typed error an toàn và fallback magenta

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `crates/ifol-gpu/tests/shared_assets/manifests/tc10_fallback.json`
- **Graph fingerprint (FNV-1a):** `d67f074b61b2198b`
- **Mô tả test case:** Xác nhận handle BindGroup bị thiếu trả typed error không panic, sau đó render target fallback đã khai báo.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** KHÔNG KHAI BÁO
- **Asset/input:** KHÔNG KHAI BÁO
- **Chính sách input:** Dùng asset theo manifest; chưa có chuẩn hóa input canonical riêng.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** Không khai báo dạng pass
- **Số pass:** `KHÔNG ÁP DỤNG`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `Không khai báo`
- **Sampler contract:** `Không khai báo`
- **Thứ tự layer kỳ vọng:** `Không khai báo`
- **Graph resources:** nodes=`0`, draw commands=`0`, tổng instances=`0`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `{"type": "RenderGraphValidationError::MissingBindGroup", "missing_bind_group": 999999, "panic_allowed": false, "web_validation_mode": "contract-mirror"}`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `1.5115 ms`
- **Thời gian render lần hai (warm/cache):** `0.7461 ms (746.1 µs)`
- **Số lần warm được đo:** `CHƯA GHI NHẬN`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `CHƯA GHI NHẬN`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `fallback graph execute_checked + submit queue + device.poll(Wait); không gồm validation graph lỗi, khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/desktop/tc10_fallback_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `09af421a1e859b25`
- **SHA-256:** `9c21e0929b3ea81661533efc8a88233eadc47a708fba48b2d5b32c562f36d042`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc10_fallback.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Ảnh toàn magenta đồng nhất, đúng fallback khi thiếu BindGroup; không có artifact. Desktop đã xác nhận typed error MissingBindGroup(999999) không panic.
- **Graph thực tế:** nodes=0, draw commands=0, instances=None
- **Validation thực tế:** error=RenderGraphValidationError::MissingBindGroup, handle=999999, passed=True, panic=False


## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `2.7000 ms`
- **Thời gian render lần hai (warm/cache):** `3.3000 ms`
- **Số lần warm được đo:** `CHƯA GHI NHẬN`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `CHƯA GHI NHẬN`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `fallback clear execute + submit queue + onSubmittedWorkDone; không gồm contract validation mirror, khởi tạo device và readback`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/web/tc10_fallback_web.bin`
- **Dấu vân tay raw (FNV-1a):** `09af421a1e859b25`
- **SHA-256:** `9c21e0929b3ea81661533efc8a88233eadc47a708fba48b2d5b32c562f36d042`
- **Ảnh:** ![WebGPU output](../outputs/web/tc10_fallback_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Ảnh toàn magenta đồng nhất, đúng fallback; Web mirror xác nhận cùng MissingBindGroup(999999) contract và không panic.
- **Graph thực tế:** nodes=0, draw commands=0, instances=None
- **Validation contract mirror:** error=RenderGraphValidationError::MissingBindGroup, handle=999999, passed=True, panic=False


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
