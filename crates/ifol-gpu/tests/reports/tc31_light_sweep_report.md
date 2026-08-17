# Báo cáo: TC31 - Luồng sáng quét trên nhân vật

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc31_light_sweep.json`
- **Graph fingerprint (FNV-1a):** `e8c707cfcbf0e9a7`
- **Mô tả test case:** Tách nhân vật mage canonical rồi áp dụng luồng sáng xiên bằng toán học, giữ nguyên alpha.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `light_sweep.wgsl`
- **Asset/input:** `canonical_sprites_heroes.png`
- **Chính sách input:** Desktop và WebGPU dùng sprite sheet PNG canonical; không dùng decoder JPEG trong phép đo parity.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** chroma_pass (Chroma key extraction, target chroma) → sweep_pass (Diagonal light sweep, target final)
- **Số pass:** `2`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `chroma_extract_mage → diagonal_light_sweep`
- **Sampler contract:** `{"address_mode_u": "clamp-to-edge", "address_mode_v": "clamp-to-edge", "address_mode_w": "clamp-to-edge", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `chroma_pass → sweep_pass`
- **Graph resources:** nodes=`2`, draw commands=`2`, tổng instances=`2`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `3.3838 ms`
- **Thời gian render lần hai (warm/cache):** `0.9073 ms (907.3 µs)`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `73.2%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `2 pass (chroma key → diagonal light sweep) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/desktop/tc31_light_sweep_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `c1d552182ed4eff6`
- **SHA-256:** `b83e877419ee2b1480968c478ba724971db623d6d293ab72c6526430e4f947ce`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc31_light_sweep.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận nhân vật mage đã được tách nền xanh, giữ alpha trên nền xám và có vùng sáng trắng-vàng quét chéo rõ ràng ở góc 45 độ; không có ảnh đen hoặc artefact ngoài mô tả.
- **Graph thực tế:** nodes=2, draw commands=2, instances=2



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `20.0000 ms`
- **Thời gian render lần hai (warm/cache):** `3.4000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `83.0%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `2 pass (chroma key → diagonal light sweep) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/web/tc31_light_sweep_web.bin`
- **Dấu vân tay raw (FNV-1a):** `c1d552182ed4eff6`
- **SHA-256:** `b83e877419ee2b1480968c478ba724971db623d6d293ab72c6526430e4f947ce`
- **Ảnh:** ![WebGPU output](../outputs/web/tc31_light_sweep_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận nhân vật mage đã được tách nền xanh, giữ alpha trên nền xám và có vùng sáng trắng-vàng quét chéo rõ ràng ở góc 45 độ; không có ảnh đen hoặc artefact ngoài mô tả.
- **Graph thực tế:** nodes=2, draw commands=2, instances=2



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
