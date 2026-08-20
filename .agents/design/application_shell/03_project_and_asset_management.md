# Project Tự Chứa, VFS Và Asset Boundary

Project phải mở, chạy và render được bằng CLI trên host có đủ feature/capability,
không phụ thuộc UI hoặc đường dẫn tuyệt đối của máy đã tạo project.

---

## 1. Cấu Trúc Project Tự Chứa (Self-Contained Project)
Project là một logical container được đọc/ghi qua `ProjectStorage`. Directory,
memory, archive hoặc remote stream chỉ là backend adapter; engine không chốt một
đuôi file hay định dạng đóng gói cụ thể. Một backend có thể biểu diễn cấu trúc:

```text
project storage (directory/archive/memory/remote)
├── project.toml           (format version, required packages, entry scene)
├── package.lock           (resolved package IDs, versions, fingerprints)
├── packages/              (optional project-local packages)
├── scenes/
│   ├── main.scene         (component records theo stable TypeId/schema version)
│   └── intro.scene
└── runtime/
    ├── ifol.asset/        (chỉ tồn tại khi package asset claim namespace)
    ├── ifol.render/       (chỉ tồn tại khi package render claim namespace)
    └── <package-id>/      (opaque package-owned data)
```

Project không dump trực tiếp Rust struct layout của ECS. Scene document chứa
record versioned; Component Registry deserialize/migrate chúng sau khi required
feature packages đã được resolve.

## 2. Namespace Và Đường Dẫn Tương Đối

Engine không biết `assets/`. Nó chỉ validate virtual path và quyền sở hữu
`runtime/<package-id>`. Nếu `pkg-asset` được cài, package đó có thể ingest source,
quản lý `AssetId`, catalog, revision và cache trong namespace của nó. Component
feature giữ stable ID do package asset định nghĩa, không giữ OS path hoặc URL tạm.

Package khác có thể định nghĩa dữ liệu hoàn toàn khác mà không sửa project core.

## 3. Hệ Thống VFS (Virtual File System)

ECS không đọc file. Engine project module dùng storage/VFS interface cho phần
manifest/lock/scenes. Package dùng scoped VFS view chỉ trong namespace đã claim;
importer/decoder nhận bytes/stream từ package owner.

### 3.1. Chế độ Desktop App (Tauri)
* VFS backend có thể đọc archive trực tiếp hoặc materialize phần cần thiết vào
  thư mục cache/tạm do host/backend quản lý.
* Feature/ECS system chỉ request `AssetId`; không gọi `std::fs`.

### 3.2. Chế độ Web App
* Web backend có thể dùng memory, IndexedDB, File System Access API hoặc stream
  từ archive tùy capability.
* `blob:` URL chỉ là chi tiết adapter khi API web cụ thể yêu cầu; không phải VFS
  contract chung.
* `ifol-gpu` không đọc asset URL/file. Decoder/importer nhận bytes/stream; Render
  Feature chuyển artifact đã chuẩn bị thành resource/command GPU.

---

## 4. Project Core Và Package-Owned Subsystem

| Service | Trách nhiệm |
|---|---|
| Engine project module | bundle, manifest, lock, scene documents, generic snapshot/load/save |
| Package namespace owner | schema, migration, source/artifact/cache policy của namespace |

`pkg-asset` là optional và dùng cùng package contract như package khác. Project
chỉ có Shape không tạo namespace asset. Decode media, font shaping và 3D parsing
không bị hard-code vào engine hoặc asset core; package đăng ký subsystem/processor
theo nhu cầu.

---

## 5. Required Features Và Portability

Manifest lưu `PackageId + version range`, không lưu đường dẫn implementation
tuyệt đối. Engine resolve package phù hợp platform trước khi load scene. Nếu
thiếu owner package, strict mode trả typed error; preservation mode giữ component
record/namespace opaque để tránh mất dữ liệu khi save lại.

Package phải đánh dấu dữ liệu cache có thể tái tạo. Project đúng không được phụ
thuộc bắt buộc vào cache sinh riêng cho một GPU/OS/codec backend.
