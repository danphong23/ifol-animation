# Báo cáo: TC28 - Gợn sóng và biến dạng xung kích

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc28_ripple.json`
- **Graph fingerprint (FNV-1a):** `01228a36813398ce`
- **Mô tả test case:** Biến dạng nền thành phố bằng gợn sóng tỏa tâm với dịch chuyển sin xác định và suy giảm theo khoảng cách.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `ripple.wgsl`
- **Asset/input:** `canonical_bg_anime_city.png`
- **Chính sách input:** Dùng fixture PNG canonical `canonical_bg_anime_city.png`, được materialize một lần từ source JPEG `bg_anime_city.jpg`; Desktop và Web nạp cùng input bytes, không dùng decoder JPEG trong phép đo parity.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** ripple_scene (City radial ripple, target final)
- **Số pass:** `1`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `city_ripple`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `ripple_scene`
- **Graph resources:** nodes=`1`, draw commands=`1`, tổng instances=`1`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `4.5936 ms`
- **Thời gian render lần hai (warm/cache):** `0.9120 ms (912.0 µs)`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `80.1%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `1 pass (radial ripple UV distortion) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/desktop/tc28_ripple_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `d6191b4350e14d66`
- **SHA-256:** `9303b065a2d7687facfe91a3bc4c8b2b64fd7a90391c5a6db225e0a99bb32821`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc28_ripple.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận nền thành phố đầy đủ và nhận diện được, biến dạng gợn sóng tỏa từ tâm đúng mô tả; không có ảnh đen hoặc artefact bất thường.
- **Graph thực tế:** nodes=1, draw commands=1, instances=1



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `9.7000 ms`
- **Thời gian render lần hai (warm/cache):** `3.1000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `68.0%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `1 pass (radial ripple UV distortion) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/web/tc28_ripple_web.bin`
- **Dấu vân tay raw (FNV-1a):** `532f0cb48a4c7b77`
- **SHA-256:** `b32ac40339530c6e3e7c1cebf39756b899b7178bf3a8805e32fc1ad5479671f2`
- **Ảnh:** ![WebGPU output](../outputs/web/tc28_ripple_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận output Web trùng cấu trúc Desktop; nền thành phố và biến dạng gợn sóng đúng mô tả, không có ảnh đen hoặc artefact bất thường.
- **Graph thực tế:** nodes=1, draw commands=1, instances=1



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `15` |
| Số pixel khác nhau | `15` |
| Sai số kênh màu lớn nhất | `1/255` |
| Khác biệt màu/presentation | `CÓ - cần theo dõi để đạt byte parity` |
| Số pixel non-background Desktop/Web | `KHÔNG ÁP DỤNG` |
| Bounding box Desktop | `KHÔNG ÁP DỤNG` |
| Bounding box WebGPU | `KHÔNG ÁP DỤNG` |
| Bounding box non-background giống nhau | `ĐẠT` |
| Số pixel mask khác nhau | `0` (ngưỡng `50000`) |
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
