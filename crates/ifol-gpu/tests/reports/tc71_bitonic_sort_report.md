# Báo cáo: TC71 - Sắp xếp Bitonic particle trên GPU

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `crates/ifol-gpu/tests/shared_assets/manifests/tc71_bitonic_sort.json`
- **Graph fingerprint (FNV-1a):** `9b46ae200653789f`
- **Mô tả test case:** Sắp xếp 65.536 particle theo depth qua 136 stage Bitonic rồi render các instance đã sắp xếp.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `compute_bitonic_sort.wgsl`, `render_bitonic_sort.wgsl`
- **Asset/input:** KHÔNG KHAI BÁO
- **Chính sách input:** Desktop và WebGPU tạo cùng vị trí, depth và màu deterministic; uniform sort parameters dùng slot canonical 256 byte.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** sort_pass (136-stage bitonic depth sort, target particle_buffer) → render_pass (Sorted particle instances, target final)
- **Số pass:** `2`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `bitonic_sort → render_sorted_particles`
- **Sampler contract:** `Không khai báo`
- **Thứ tự layer kỳ vọng:** `sort_pass → render_pass`
- **Graph resources:** nodes=`2`, draw commands=`137`, tổng instances=`65536`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `106.7969 ms`
- **Thời gian render lần hai (warm/cache):** `97.2239 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Warm diff chi tiết:** `bytes=0, pixels=0, max_delta=0/255, tolerance=0`
- **Speedup cold → warm:** `9.0%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `execute_checked + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; state mutable được reset hoặc ghi đè trước warm; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/desktop/tc71_bitonic_sort_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `2fcca40a87e4c1f8`
- **SHA-256:** `0f20d288b8e63feba8c00a3bafbd12d89fb296c180cda5179bc4f44f71c8979c`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc71_bitonic_sort.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** TC71: Desktop hiển thị trường particle dày màu đỏ/hồng; depth tăng dần đã được kiểm chứng bằng numeric validation.
 - **Numeric validation:** `{"first_depth": 0.0, "last_depth": 0.9990000128746033, "particle_count": 65536, "seed_reset_before_warm": true, "sorted_non_decreasing": true, "stage_count": 136}`
- **Graph thực tế:** nodes=2, draw commands=137, instances=65536



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `83.2000 ms`
- **Thời gian render lần hai (warm/cache):** `81.6000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Warm diff chi tiết:** `bytes=0, pixels=0, max_delta=0/255, tolerance=0`
- **Speedup cold → warm:** `1.9%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `136 canonical bitonic stages + 65536 instanced draw + source-to-destination ping-pong + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; state mutable được reset trước warm; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/web/tc71_bitonic_sort_web.bin`
- **Dấu vân tay raw (FNV-1a):** `cf9aa29cf1a5bda3`
- **SHA-256:** `e5043d98f19c80d2b1139f22db119a1c3873cef931c2775f992f029fbf09547f`
- **Ảnh:** ![WebGPU output](../outputs/web/tc71_bitonic_sort_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** TC71: Web hiển thị cùng bố cục particle màu đỏ/hồng; không thấy vùng trống hay fallback.
 - **Numeric validation:** `{"particle_count": 65536, "stage_count": 136, "sorted_non_decreasing": true, "first_depth": 0, "last_depth": 0.9990000128746033, "seed_reset_before_warm": true}`
- **Graph thực tế:** nodes=2, draw commands=137, instances=65536



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `15` |
| Số pixel khác nhau | `15` |
| Sai số kênh màu lớn nhất | `2/255` |
| Khác biệt màu/presentation | `CÓ - cần theo dõi để đạt byte parity` |
| Số pixel non-background Desktop/Web | `480000 / 480000` |
| Bounding box Desktop | `(0, 0, 799, 599)` |
| Bounding box WebGPU | `(0, 0, 799, 599)` |
| Bounding box non-background giống nhau | `ĐẠT` |
| Số pixel mask khác nhau | `0` (ngưỡng `120000`) |
| Parity cấu trúc không phụ thuộc màu | `ĐẠT` |
| Cache giữ nguyên output cold/warm ở cả hai môi trường | `ĐẠT` |
| Warm diff chi tiết đã được ghi nhận | `Desktop: bytes=0, pixels=0, max_delta=0/255, tolerance=0; WebGPU: bytes=0, pixels=0, max_delta=0/255, tolerance=0` |
| Validation/fallback contract không panic | `ĐẠT` |
| Đúng mô tả test case | `ĐẠT` |

**Kết luận:** `ĐẠT CÓ ĐIỀU KIỆN - graph và cấu trúc render giống; khác biệt còn lại thuộc pixel/màu và nằm trong ngưỡng đã khai báo.`

## 5. Phân tích hiệu suất

Các giá trị trên đo thời gian thực thi graph, submit lệnh và chờ GPU hoàn tất;
không bao gồm khởi tạo device/pipeline hoặc readback. Vì vậy `cold` ở đây là
lần execute đầu sau khi resource/pipeline đã được tạo, không phải cold start
của toàn bộ ứng dụng. Giá trị dưới `1 ms` tương đương microsecond và cần được
đọc theo đơn vị đó khi phân tích.
