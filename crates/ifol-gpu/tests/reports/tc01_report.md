# Báo cáo: TC01 - Render rỗng

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc01_empty.json`
- **Graph fingerprint (FNV-1a):** `c34f7f31847194c6`
- **Mô tả test case:** Xóa (clear) target offscreen 800x600 thành một màu xám nhạt đồng nhất, không có điểm ảnh rác.
- **Target:** `800x600`, `Rgba8Unorm`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `4.5663 ms`
- **Thời gian render lần hai (warm/cache):** `0.4775 ms (477.5 µs)`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `execute_checked + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/desktop/tc01_empty_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `56e43abaf9ecc325`
- **SHA-256:** `d64fc378fbb5847dd03659fcf2681adfc74ff4b3df8d4794dea3dbf88704db5f`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc01_empty.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: toàn bộ target là màu xám đồng nhất, không có điểm ảnh rác.

## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `2.2000 ms`
- **Thời gian render lần hai (warm/cache):** `1.1000 ms`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `execute offscreen + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/web/tc01_empty_web.bin`
- **Dấu vân tay raw (FNV-1a):** `56e43abaf9ecc325`
- **SHA-256:** `d64fc378fbb5847dd03659fcf2681adfc74ff4b3df8d4794dea3dbf88704db5f`
- **Ảnh:** ![WebGPU output](../outputs/web/tc01_empty_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: toàn bộ target là màu xám đồng nhất, không có điểm ảnh rác.

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
| Đúng mô tả test case | `ĐẠT` |

**Kết luận:** `ĐẠT - output giống tuyệt đối từng byte.`

## 5. Phân tích hiệu suất

Các giá trị trên đo thời gian thực thi graph, submit lệnh và chờ GPU hoàn tất;
không bao gồm khởi tạo device/pipeline hoặc readback. Vì vậy `cold` ở đây là
lần execute đầu sau khi resource/pipeline đã được tạo, không phải cold start
của toàn bộ ứng dụng. Giá trị dưới `1 ms` tương đương microsecond và cần được
đọc theo đơn vị đó khi phân tích.
