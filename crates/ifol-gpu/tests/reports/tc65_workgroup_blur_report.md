# Báo cáo: TC65 - Làm mờ bằng bộ nhớ chia sẻ workgroup

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `../shared_assets/manifests/tc65_workgroup_blur.json`
- **Graph fingerprint (FNV-1a):** `9219b57bf1c71f6b`
- **Mô tả test case:** Compute kernel 9x9 lấy tile 24x24 vào workgroup memory, giữ nửa trái sắc nét và làm mờ nửa phải.
- **Target:** `800x600`, `Rgba8Unorm`
- **Shader/WGSL:** `compute_workgroup_blur.wgsl`
- **Asset/input:** `sprites_heroes.jpeg`
- **Chính sách input:** Desktop và WebGPU dùng cùng asset sprites_heroes.jpeg; decoder nền tảng chỉ là input fixture, không thuộc graph contract.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** blur_pass (Workgroup shared-memory blur, target output)
- **Số pass:** `1`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `workgroup_blur`
- **Sampler contract:** `Không khai báo`
- **Thứ tự layer kỳ vọng:** `blur_pass`
- **Graph resources:** nodes=`1`, draw commands=`1`, tổng instances=`0`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `5.9689 ms`
- **Thời gian render lần hai (warm/cache):** `1.8312 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `69.3%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `execute_checked + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; state mutable được reset trước warm; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `../outputs/desktop/tc65_workgroup_blur_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `f1d13b76ce4ff87d`
- **SHA-256:** `daeb2d87ac11c5377a55b27c5c454f9a218fe365d820af3d3301e4b4c1597a37`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc65_workgroup_blur.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Desktop hiển thị đúng nửa trái là ảnh nhân vật sắc nét, divider vàng ở giữa và nửa phải là Gaussian blur từ workgroup tile; không có vùng đen hoặc artifact cấu trúc.
- **Graph thực tế:** nodes=1, draw commands=1, instances=0



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `22.5000 ms`
- **Thời gian render lần hai (warm/cache):** `3.1000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `86.2%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `1 workgroup-shared blur dispatch 50x38 + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; state mutable được reset trước warm; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `../outputs/web/tc65_workgroup_blur_web.bin`
- **Dấu vân tay raw (FNV-1a):** `0b5e780b7dbfdf73`
- **SHA-256:** `129ada9264651c89fcb33253c58d370e8013b9910575ed29863e55f61c4834df`
- **Ảnh:** ![WebGPU output](../outputs/web/tc65_workgroup_blur_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** WebGPU hiển thị cùng bố cục ảnh gốc, divider vàng và vùng blur; khác biệt nhỏ ở biên/raster hóa, không có lỗi cấu trúc.
- **Graph thực tế:** nodes=1, draw commands=1, instances=0



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `56353` |
| Số pixel khác nhau | `43747` |
| Sai số kênh màu lớn nhất | `7/255` |
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
