# Báo cáo: TC53 - Ma trận 8 chế độ hòa trộn

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `../shared_assets/manifests/tc53_blend_modes.json`
- **Graph fingerprint (FNV-1a):** `0045bf536afcf57d`
- **Mô tả test case:** So sánh tám công thức hòa trộn deterministic trên scene canonical dạng lưới.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `blend_modes.wgsl`
- **Asset/input:** `canonical_bg_scifi.png`, `canonical_sprites_heroes.png`
- **Chính sách input:** Desktop và WebGPU dùng texture PNG canonical cho nền và atlas; không dùng decoder JPEG trong phép đo parity.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** blend_pass (4x2 blend mode matrix, target final)
- **Số pass:** `1`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `blend_matrix`
- **Sampler contract:** `{"address_mode_u": "clamp-to-edge", "address_mode_v": "clamp-to-edge", "address_mode_w": "clamp-to-edge", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `blend_pass`
- **Graph resources:** nodes=`1`, draw commands=`1`, tổng instances=`1`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `3.5300 ms`
- **Thời gian render lần hai (warm/cache):** `1.1219 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `68.2%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `1 pass 4x2 blend matrix + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `../outputs/desktop/tc53_blend_modes_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `375766d4845d7c3b`
- **SHA-256:** `85db96e5ef24a23c36e0fed8dac6b591144e114f51918f1254252b7d3a512c09`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc53_blend_modes.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Desktop hiển thị đầy đủ ma trận 4x2 gồm tám ô blend, có đường lưới trắng, nền Sci-Fi và sprite canonical; không có ô đen hay mất layer.
- **Graph thực tế:** nodes=1, draw commands=1, instances=1



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `308.2000 ms`
- **Thời gian render lần hai (warm/cache):** `3.6000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `98.8%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `1 pass 4x2 blend matrix + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `../outputs/web/tc53_blend_modes_web.bin`
- **Dấu vân tay raw (FNV-1a):** `c23271404bc161fa`
- **SHA-256:** `30a224cb2284daec1520558c0911e3fe28bb58597e10ab3c7fef6132dd9041e6`
- **Ảnh:** ![WebGPU output](../outputs/web/tc53_blend_modes_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** WebGPU hiển thị cùng ma trận 4x2, cùng tám ô blend, đường lưới trắng và sprite canonical; không có lỗi cấu trúc hoặc ô đen.
- **Graph thực tế:** nodes=1, draw commands=1, instances=1



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `69` |
| Số pixel khác nhau | `51` |
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
