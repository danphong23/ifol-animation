# Báo cáo: TC09 - Pipeline caching và tái sử dụng render bundle

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `crates/ifol-gpu/tests/shared_assets/manifests/tc09_caching.json`
- **Graph fingerprint (FNV-1a):** `dd339060999b24ed`
- **Mô tả test case:** Chạy cùng một graph một lần cold và mười lần warm, tái sử dụng pipeline, resource và graph để đo cache/bundle.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `particles_10k.wgsl`, `texture_blit.wgsl`
- **Asset/input:** `canonical_bg_nightsky.png`
- **Chính sách input:** Dùng PNG canonical để Desktop/WebGPU giải mã cùng một input byte-level.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** Không khai báo dạng pass
- **Số pass:** `KHÔNG ÁP DỤNG`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `background → particles`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "nearest", "min_filter": "nearest", "mipmap_filter": "nearest"}`
- **Thứ tự layer kỳ vọng:** `Không khai báo`
- **Graph resources:** nodes=`1`, draw commands=`2`, tổng instances=`10001`, procedural particles=`10000`
- **Node pool contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `2.2251 ms`
- **Thời gian render lần hai (warm/cache):** `0.8172 ms (817.2 µs)`
- **Số lần warm được đo:** `10`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `63.3%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `execute_checked của cùng graph + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/desktop/tc09_caching_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `895fef5150f66a69`
- **SHA-256:** `35f64aa6f4b52632d0f5e0a1b750e03cdab0d8d47f8e0ee0093e7c260cf29f63`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc09_caching.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Nền trời đêm phủ toàn khung hình với rất nhiều hạt sao procedural màu trắng, vàng và cyan; không có artifact hoặc mất draw command. Các lần warm giữ nguyên output.
- **Graph thực tế:** nodes=1, draw commands=2, instances=10000


## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `21.8000 ms`
- **Thời gian render lần hai (warm/cache):** `2.6100 ms`
- **Số lần warm được đo:** `10`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `88.0%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `execute offscreen của cùng graph + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `crates/ifol-gpu/tests/outputs/web/tc09_caching_web.bin`
- **Dấu vân tay raw (FNV-1a):** `895fef5150f66a69`
- **SHA-256:** `35f64aa6f4b52632d0f5e0a1b750e03cdab0d8d47f8e0ee0093e7c260cf29f63`
- **Ảnh:** ![WebGPU output](../outputs/web/tc09_caching_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Nền trời đêm phủ toàn khung hình với rất nhiều hạt sao procedural màu trắng, vàng và cyan; không có artifact hoặc mất draw command. Các lần warm giữ nguyên output.
- **Graph thực tế:** nodes=1, draw commands=2, instances=10000


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
| Đúng mô tả test case | `ĐẠT` |

**Kết luận:** `ĐẠT - output giống tuyệt đối từng byte.`

## 5. Phân tích hiệu suất

Các giá trị trên đo thời gian thực thi graph, submit lệnh và chờ GPU hoàn tất;
không bao gồm khởi tạo device/pipeline hoặc readback. Vì vậy `cold` ở đây là
lần execute đầu sau khi resource/pipeline đã được tạo, không phải cold start
của toàn bộ ứng dụng. Giá trị dưới `1 ms` tương đương microsecond và cần được
đọc theo đơn vị đó khi phân tích.
