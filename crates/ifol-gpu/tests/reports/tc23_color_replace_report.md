# Báo cáo: TC23 - Thay bảng màu bằng dịch chuyển HSV

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc23_color_replace.json`
- **Graph fingerprint (FNV-1a):** `5e6dcfeb32712bc9`
- **Mô tả test case:** Thay bảng màu hồng của sprite canonical thành cyan, giữ value/shading và loại phông xanh.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `color_replace.wgsl`
- **Asset/input:** `canonical_sprites_heroes.png`
- **Chính sách input:** Dùng PNG canonical để Desktop/WebGPU giải mã cùng một input byte-level.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** replace_scene (Cyan palette replacement, target final)
- **Số pass:** `1`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `palette_swap`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `replace_scene`
- **Graph resources:** nodes=`1`, draw commands=`1`, tổng instances=`1`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `3.0819 ms`
- **Thời gian render lần hai (warm/cache):** `0.8330 ms (833.0 µs)`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `73.0%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `1 pass (HSV palette replacement + chroma key) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/desktop/tc23_color_replace_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `e576bb3cbe56c302`
- **SHA-256:** `9ff1adffb6d532fb5138be3217fed6b2cf3ef21131db2d8a8694992621d34a3e`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc23_color_replace.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận sprite đổi từ hồng/tím sang cyan; shading và highlight vẫn được giữ; nền xanh bị loại bỏ; không có vùng đen hoặc artefact bất thường.
- **Graph thực tế:** nodes=1, draw commands=1, instances=1



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `15.9000 ms`
- **Thời gian render lần hai (warm/cache):** `3.3000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `79.2%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `1 pass (HSV palette replacement + chroma key) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/web/tc23_color_replace_web.bin`
- **Dấu vân tay raw (FNV-1a):** `e576bb3cbe56c302`
- **SHA-256:** `9ff1adffb6d532fb5138be3217fed6b2cf3ef21131db2d8a8694992621d34a3e`
- **Ảnh:** ![WebGPU output](../outputs/web/tc23_color_replace_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận output web trùng desktop; sprite đổi sang cyan, shading/highlight được giữ, nền xanh bị loại bỏ; không có vùng đen hoặc artefact bất thường.
- **Graph thực tế:** nodes=1, draw commands=1, instances=1



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
