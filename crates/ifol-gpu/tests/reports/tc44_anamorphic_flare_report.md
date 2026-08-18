# Báo cáo: TC44 - Tia flare anamorphic

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/shared_assets/manifests/tc44_anamorphic_flare.json`
- **Graph fingerprint (FNV-1a):** `8acb587266daa9a5`
- **Mô tả test case:** Lấy mẫu 33 tap theo trục ngang, lọc điểm sáng và tạo streak xanh anamorphic.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `anamorphic_flare.wgsl`
- **Asset/input:** `canonical_bg_scifi.png`
- **Chính sách input:** Desktop và WebGPU dùng canonical_bg_scifi.png; không dùng decoder JPEG trong phép đo parity.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** flare_pass (không tên, target final)
- **Số pass:** `1`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `anamorphic_flare`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `flare_pass`
- **Graph resources:** nodes=`1`, draw commands=`1`, tổng instances=`1`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `3.7100 ms`
- **Thời gian render lần hai (warm/cache):** `1.7704 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `52.3%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `1 pass effect + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/outputs/desktop/tc44_anamorphic_flare_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `35c089f6ddab59e5`
- **SHA-256:** `cbd3b04fbf73ffe4b96ae9e26ba2509285f0757d346cf25ca260d07db43c28e9`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc44_anamorphic_flare.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Cảnh Sci-Fi có streak ngang xanh lam từ vùng sáng; không có ảnh đen.
- **Graph thực tế:** nodes=1, draw commands=1, instances=1



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `7.7000 ms`
- **Thời gian render lần hai (warm/cache):** `4.3000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `44.2%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `1 pass (anamorphic horizontal flare) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/outputs/web/tc44_anamorphic_flare_web.bin`
- **Dấu vân tay raw (FNV-1a):** `0535e72f6d92d5b4`
- **SHA-256:** `342487e24760b282a0f048d3ba453b7b7abd4b289d8eb2a13f05effb4ece894c`
- **Ảnh:** ![WebGPU output](../outputs/web/tc44_anamorphic_flare_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Cảnh Sci-Fi có streak ngang xanh lam từ vùng sáng; không có ảnh đen.
- **Graph thực tế:** nodes=1, draw commands=1, instances=1



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `113` |
| Số pixel khác nhau | `106` |
| Sai số kênh màu lớn nhất | `77/255` |
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
