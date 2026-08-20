# Quản Lý Phiên Bản & Nâng Cấp Dữ Liệu (Versioning & Migrations)

Bài toán đau đầu nhất của các phần mềm làm việc với File: Người dùng đang lưu project bằng phiên bản V1. Sau 1 năm, phần mềm update lên V2 (cấu trúc Component trong ECS thay đổi, đổi tên trường dữ liệu). Làm sao để mở lại file V1 mà không bị lỗi (Crash)?

---

## 1. Nguyên Tắc Tách Biệt Bộ Nhớ Và Ổ Cứng
*   **Dữ liệu ổ cứng:** Manifest, package lock, scene records và package-owned
    namespace records có version ổn định.
*   **Dữ liệu RAM (ECS Runtime State):** Là cấu trúc Rust struct mới nhất, tối ưu nhất đang chạy trong máy.
*   **Không bao giờ Mapping trực tiếp 1-1** từ File thẳng vào ECS Struct mà không qua màng lọc.

Project format version, scene format version, package version và component schema
version là các khái niệm khác nhau; không dùng một số `version` duy nhất để đại
diện tất cả.

## 2. Đường Ống Migration (Upgrader Pipeline)
Manifest và component record tối thiểu có version/ID ổn định:
```json
{
  "format_version": 1,
    "required_packages": [
    { "id": "ifol.shape", "version": "^1" }
  ]
}
```

```json
{
  "entity_id": "...",
  "component_type": "ifol.shape",
  "schema_version": 2,
  "data": {}
}
```

Khi engine project module mở project:

1. migrate container/manifest format nếu cần;
2. resolve required packages và versions;
3. đăng ký component schemas/migration handlers;
4. migrate từng scene/component record theo owner feature;
5. validate dữ liệu chuẩn hóa;
6. mới deserialize thành runtime component và đưa vào ECS World.

Migration của `ShapeComponent` thuộc package shape, không thuộc ECS kernel hoặc
engine. Engine project module chỉ điều phối chain và transaction.

Migration một chiều giúp giữ compatibility có chủ đích, nhưng không được tuyên bố
"luôn mở mọi project cổ" nếu migration/package cần thiết không còn khả dụng.
Loader phải trả typed error và giữ opaque component record khi thiếu feature để
không làm mất dữ liệu lúc save lại.

---

## 3. Quy Tắc Versioning Feature

- `FeatureId` và component type ID không đổi theo rename thư mục/crate.
- Breaking schema change phải tăng schema version và cung cấp migration hoặc
  khai báo rõ unsupported path.
- Project tham chiếu version range; lock metadata có thể ghi implementation đã
  dùng nhưng không lưu đường dẫn tuyệt đối.
- Migration phải deterministic, test bằng golden fixtures và không phụ thuộc UI.
