# Báo cáo: TC61 - Tính toán storage buffer bằng compute

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `../shared_assets/manifests/tc61_compute_buffer_math.json`
- **Graph fingerprint (FNV-1a):** `91a37c1c43c4f64c`
- **Mô tả test case:** Tính song song 10.240 vec4 bằng compute, đối chiếu công thức CPU rồi render đồ thị A, B và C.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `compute_buffer_math.wgsl`, `compute_plot.wgsl`
- **Asset/input:** KHÔNG KHAI BÁO
- **Chính sách input:** Desktop và WebGPU tự tạo cùng mảng f32 xác định trong test; không dùng decoder texture cho phần số học.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** compute_pass (Storage buffer arithmetic, target buffer_c) → plot_pass (A/B/C plot, target final)
- **Số pass:** `2`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `compute_math → compute_plot`
- **Sampler contract:** `Không khai báo`
- **Thứ tự layer kỳ vọng:** `compute_pass → plot_pass`
- **Graph resources:** nodes=`2`, draw commands=`2`, tổng instances=`1`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `2.7937 ms`
- **Thời gian render lần hai (warm/cache):** `0.6827 ms (682.7 µs)`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `75.6%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `compute dispatch + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `../outputs/desktop/tc61_compute_buffer_math_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `35eb963c444f2721`
- **SHA-256:** `cfef64d43e15e1893b09f492dfbfbb7f30ea381aac2c28af1e16ab1b2124d80b`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc61_compute_buffer_math.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Desktop hiển thị đúng đồ thị grid với Input A màu vàng, Input B màu cam và Output C màu cyan; numeric readback khớp CPU 10.240/10.240 phần tử, sai số cực đại 0,00005054.
- **Graph thực tế:** nodes=2, draw commands=2, instances=1



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `162.6000 ms`
- **Thời gian render lần hai (warm/cache):** `3.5000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `97.8%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `1 compute dispatch + 1 plot pass + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `../outputs/web/tc61_compute_buffer_math_web.bin`
- **Dấu vân tay raw (FNV-1a):** `8b5eb0599fd03b9b`
- **SHA-256:** `fc8db5f6fbadc7dffcb26775e497ccfc42bdb6c153dded0426465e3a20917713`
- **Ảnh:** ![WebGPU output](../outputs/web/tc61_compute_buffer_math_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** WebGPU hiển thị cùng đồ thị và ba đường dữ liệu; compute dispatch hoàn tất không validation error, hình học/đường cong trùng về cấu trúc, sai khác chỉ 1 mức kênh ở 49 pixel.
- **Graph thực tế:** nodes=2, draw commands=2, instances=1



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `52` |
| Số pixel khác nhau | `49` |
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
