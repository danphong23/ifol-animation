# Báo cáo: TC16 - Hình SDF 2D và vector graphics

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc16_sdf.json`
- **Graph fingerprint (FNV-1a):** `8962fd4fa969ea29`
- **Mô tả test case:** Render bốn hình Signed Distance Field xác định với khử răng cưa mượt, viền và hiệu ứng glow.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `sdf_shapes.wgsl`
- **Asset/input:** KHÔNG KHAI BÁO
- **Chính sách input:** Không dùng texture/asset; input là uniform và graph canonical từ manifest.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** sdf_scene (2D SDF scene, target final)
- **Số pass:** `1`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `circle → rounded_rect → ring → triangle`
- **Sampler contract:** `Không khai báo`
- **Thứ tự layer kỳ vọng:** `sdf_scene`
- **Graph resources:** nodes=`1`, draw commands=`4`, tổng instances=`4`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `2.5806 ms`
- **Thời gian render lần hai (warm/cache):** `0.8089 ms (808.9 µs)`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `68.7%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `1 pass (2D SDF scene, 4 draw commands) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/desktop/tc16_sdf_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `8a5b893b64047f8a`
- **SHA-256:** `3665605cd69baac8e0e8ba39d999365b0f046295becdca1f15f80c8e2067bdb3`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc16_sdf.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận bốn hình SDF đúng mô tả: mặt trời đỏ, thẻ bo góc xanh, vòng neon xanh lục và nút play tím; viền, glow và anti-aliasing rõ, không có artefact lớn.
- **Graph thực tế:** nodes=1, draw commands=4, instances=4



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `10.4000 ms`
- **Thời gian render lần hai (warm/cache):** `3.6000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `65.4%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `1 pass (2D SDF scene, 4 draw commands) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/web/tc16_sdf_web.bin`
- **Dấu vân tay raw (FNV-1a):** `ac26d35ec25991a3`
- **SHA-256:** `6e39dabed796e1bf9d8ee0b9d7f851b9ceadbf24bd35c1a24d89f284f7186664`
- **Ảnh:** ![WebGPU output](../outputs/web/tc16_sdf_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận bốn hình SDF đúng mô tả: mặt trời đỏ, thẻ bo góc xanh, vòng neon xanh lục và nút play tím; viền, glow và anti-aliasing rõ, không có artefact lớn.
- **Graph thực tế:** nodes=1, draw commands=4, instances=4



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `1` |
| Số pixel khác nhau | `1` |
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

## 6. Phân tích sai khác raw theo tọa độ

Raw Desktop/Web có cùng kích thước `1.920.000` byte. Chỉ có `1` byte thuộc
`1` pixel khác nhau, sai số lớn nhất là `1/255` và alpha không thay đổi:

```text
(600,458) Desktop RGBA (177,131,178,255) → Web RGBA (177,131,179,255)
```

Sai khác đơn lẻ nằm ở biên glow của triangle, không làm thay đổi hình học,
mask hoặc bố cục. Kết quả đạt parity về graph, cấu trúc và nội dung thị giác;
byte parity còn phụ thuộc sai số lượng tử hóa giữa backend GPU.

Web cold `10.4000 ms` và warm `3.6000 ms` đều đo cùng phạm vi một pass; cold
cao hơn do chi phí submit/biên dịch lười của lần chạy đầu, không phải do graph
khác với Desktop.
