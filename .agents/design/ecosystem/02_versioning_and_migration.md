# Quản Lý Phiên Bản & Nâng Cấp Dữ Liệu (Versioning & Migrations)

Bài toán đau đầu nhất của các phần mềm làm việc với File: Người dùng đang lưu project bằng phiên bản V1. Sau 1 năm, phần mềm update lên V2 (cấu trúc Component trong ECS thay đổi, đổi tên trường dữ liệu). Làm sao để mở lại file V1 mà không bị lỗi (Crash)?

---

## 1. Nguyên Tắc Tách Biệt Bộ Nhớ và Ổ Cứng
*   **Dữ liệu Ổ cứng (Disk State - `project.json`):** Là định dạng lưu trữ cứng nhắc, có Version.
*   **Dữ liệu RAM (ECS Runtime State):** Là cấu trúc Rust struct mới nhất, tối ưu nhất đang chạy trong máy. 
*   **Không bao giờ Mapping trực tiếp 1-1** từ File thẳng vào ECS Struct mà không qua màng lọc.

## 2. Đường Ống Migration (Upgrader Pipeline)
Mỗi file `project.json` bên trong `.ifol` bắt buộc phải có thuộc tính `version`:
```json
{
  "version": 1,
  "entities": [...]
}
```

Khi Asset Manager đọc file, nó đi qua một chuỗi các trạm kiểm duyệt (Migration Chain):
1.  Đọc file thấy `version: 1`. Nhưng phần mềm hiện tại đang chạy lõi ECS V3.
2.  Chạy hàm `migrate_v1_to_v2(json_data)`: Hàm này tìm các Component đổi tên, thêm các Component bắt buộc bị thiếu, xóa các giá trị lỗi thời. Kết quả ra cục JSON V2.
3.  Tiếp tục chạy hàm `migrate_v2_to_v3(json_data)`. Kết quả ra cục JSON chuẩn V3.
4.  Cuối cùng, lấy JSON V3 đó chuyển thành dữ liệu đưa vào ECS.

👉 Nhờ chuỗi Migration một chiều này, phần mềm **luôn có khả năng tương thích ngược (Backward Compatibility)** với bất kỳ file project cổ đại nào, mà lõi ECS hiện tại không cần phải chứa những dòng code "vá lỗi" rác rưởi của quá khứ.
