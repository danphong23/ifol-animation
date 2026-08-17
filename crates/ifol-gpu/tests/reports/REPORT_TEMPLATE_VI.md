# Mẫu báo cáo kiểm thử parity Desktop/WebGPU — TCXX

> Mỗi test case phải có đúng một báo cáo theo mẫu này. Không điền kết quả Web
> bằng suy luận từ Desktop; nếu chưa chạy phải ghi rõ `CHƯA CHẠY`.

## 1. Thông tin test case

- **Mã test:** `TCXX`
- **Tên:**
- **Mục tiêu kiểm thử:**
- **Mô tả expected output:**
- **Ngày chạy:**
- **Commit/source:**

## 2. Graph và input dùng chung

- **Manifest dùng chung:**
- **Graph fingerprint:**
- **Shader/WGSL:**
- **Asset/input:**
- **Seed/tham số cố định:**
- **Desktop và Web có dùng cùng manifest/graph contract:** `ĐẠT / KHÔNG ĐẠT`
- **Khác biệt implementation có chủ ý:**

## 3. Môi trường Desktop

- **Adapter/backend:**
- **Kích thước target:**
- **Texture format:**
- **Thời gian render lần đầu (cold):**
- **Thời gian render lần hai (warm/cache):**
- **Thời gian readback riêng:**
- **Raw hash:**
- **Ảnh kết quả:**
- **Lỗi GPU/validation:**

### Đánh giá ảnh Desktop

- **Đúng mô tả test case:** `ĐẠT / KHÔNG ĐẠT / CẦN XEM LẠI`
- **Đúng graph:**
- **Checklist vision:**
  - Bố cục/vị trí:
  - Hình dạng/layer/depth:
  - Artifact/rác/viền:
  - Alpha/blend:
  - Màu sắc nếu test có yêu cầu màu:
- **Nhận xét vision:**

## 4. Môi trường WebGPU

- **Adapter/browser:**
- **Kích thước target:**
- **Texture format:**
- **Thời gian render lần đầu (cold):**
- **Thời gian render lần hai (warm/cache):**
- **Thời gian readback riêng:**
- **Raw hash:**
- **Ảnh kết quả:**
- **Lỗi GPU/validation:**

### Đánh giá ảnh WebGPU

- **Đúng mô tả test case:** `ĐẠT / KHÔNG ĐẠT / CẦN XEM LẠI`
- **Đúng graph:**
- **Checklist vision:**
  - Bố cục/vị trí:
  - Hình dạng/layer/depth:
  - Artifact/rác/viền:
  - Alpha/blend:
  - Màu sắc nếu test có yêu cầu màu:
- **Nhận xét vision:**

## 5. So sánh Desktop và WebGPU

| Tiêu chí | Kết quả |
| --- | --- |
| Manifest/graph fingerprint | |
| Shader và input | |
| Kích thước/format canonical | |
| Raw byte giống tuyệt đối | |
| Số byte khác nhau | |
| Sai số pixel lớn nhất | |
| Khác biệt chỉ do presentation/color policy | |
| Parity kết luận | `ĐẠT / KHÔNG ĐẠT / CẦN XEM LẠI` |

## 6. Phân tích hiệu suất

> Timing phải ghi rõ đang đo `execute + submit + GPU wait` hay đo cả khởi tạo
> device/pipeline/readback. Không gọi cold render là cold start của toàn bộ app
> nếu device/pipeline đã được tạo trước khi bắt đầu timer.

- **Desktop:** cold / warm / readback:
- **WebGPU:** cold / warm / readback:
- **Đơn vị chuẩn:** ms; giá trị dưới `1 ms` ghi thêm µs để dễ đọc.
- **Giải thích cache:** pipeline/bundle/resource nào đã warm:
- **Có dấu hiệu regression:** `CÓ / KHÔNG`
- **Nhận xét:**

## 7. Kết luận

- **Đúng mô tả test case:**
- **Đúng graph:**
- **Desktop/Web parity:**
- **Hiệu suất:**
- **Vấn đề còn lại:**
- **Trạng thái:** `ĐẠT / KHÔNG ĐẠT / CHƯA ĐỦ BẰNG CHỨNG`
