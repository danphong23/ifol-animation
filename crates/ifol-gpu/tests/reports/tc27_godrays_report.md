# Báo cáo: TC27 - Tia sáng thể tích

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc27_godrays.json`
- **Graph fingerprint (FNV-1a):** `39041e2d99fd185f`
- **Mô tả test case:** Áp dụng vòng lặp tích lũy tia sáng tỏa tâm xác định lên nền rừng canonical.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `godrays.wgsl`
- **Asset/input:** `canonical_bg_forest.png`
- **Chính sách input:** Dùng PNG canonical để Desktop/WebGPU giải mã cùng một input byte-level.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** godrays_scene (Forest volumetric light shafts, target final)
- **Số pass:** `1`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `forest_godrays`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `godrays_scene`
- **Graph resources:** nodes=`1`, draw commands=`1`, tổng instances=`1`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `4.0286 ms`
- **Thời gian render lần hai (warm/cache):** `2.8671 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `28.8%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `1 pass (100-sample radial godrays accumulation) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/desktop/tc27_godrays_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `cf7e66cc55d491c9`
- **SHA-256:** `f115f2c2b0af860fcea6653a9a14e0b27d1d8d7dcd7e08222145f719c9153921`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc27_godrays.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận nền rừng vẫn rõ, các tia sáng tỏa từ vùng sáng phía trên giữa và hiệu ứng volumetric light shafts đúng mô tả; không có ảnh đen hoặc artefact bất thường.
- **Graph thực tế:** nodes=1, draw commands=1, instances=1



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `11.5000 ms`
- **Thời gian render lần hai (warm/cache):** `3.3000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `71.3%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `1 pass (100-sample radial godrays accumulation) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/web/tc27_godrays_web.bin`
- **Dấu vân tay raw (FNV-1a):** `520a65af65d6495a`
- **SHA-256:** `5565fd506219f365fc72050b3c71dbb7c1bfcb83a4d4d1e4cbaf157dbfa15a3a`
- **Ảnh:** ![WebGPU output](../outputs/web/tc27_godrays_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận output Web có cùng bố cục và hiệu ứng tia sáng với Desktop; nền rừng rõ, vùng sáng và tia tỏa tâm đúng mô tả, không có ảnh đen hoặc artefact bất thường.
- **Graph thực tế:** nodes=1, draw commands=1, instances=1



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `33` |
| Số pixel khác nhau | `33` |
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
