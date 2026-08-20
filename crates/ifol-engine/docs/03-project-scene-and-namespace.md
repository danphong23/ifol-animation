# Project, scene và package namespace

## 1. Phần engine quản lý

```text
project/
├── project.toml
├── package.lock
├── packages/
├── scenes/
└── runtime/
    └── <package-id>/
```

- `project.toml`: format version, required package constraints, entry scene và
  generic engine settings;
- `package.lock`: package identity/version/fingerprint đã resolve;
- `packages/`: optional project-local package distribution;
- `scenes/`: generic entity/component records;
- `runtime/<package-id>`: opaque namespace do package claim.

Engine không mặc định tạo `assets`, `presets`, `render`, `animation`, `game` hoặc
`artifacts`. Package owner tự định nghĩa dữ liệu, cache policy và migration trong
namespace của nó.

Khi project được mở, `required_packages` là root selection. Package candidate
không nằm trong transitive closure của các root không được activate. Nếu có
`package.lock`, lock phải khớp chính xác với closure đã resolve; engine không tự
thay package hoặc âm thầm cập nhật lock trong lúc build.

## 2. Scene document

```text
Scene session (`SceneId` supplied by runtime)
└── SceneDocument
    ├── stable serialized EntityKey
    ├── component records
    │   ├── stable SchemaId
    │   ├── schema version
    │   └── payload
    └── opaque records chưa có owner package
```

Serialized `EntityKey` không phải raw runtime `EntityId`; loader tạo mapping mới.
Engine không deserialize payload khi owner codec chưa đăng ký.

## 3. Load transaction

```text
read manifest
  -> resolve/lock packages
  -> register schemas/migrations/runtime
  -> read scene records
  -> migrate + validate owned records
  -> preserve unknown records opaque
  -> allocate entities
  -> attach components
  -> validate references/hierarchy theo owner package
  -> publish active scene session
```

Load lỗi không để lại half-loaded world. Missing package có policy explicit:
strict open trả lỗi, còn preservation mode giữ opaque data nhưng không kích hoạt
semantic thiếu owner.

## 4. Save và snapshot

Save không dump layout Rust/ECS. Package codec chuyển runtime component thành
versioned record. Unknown opaque record được giữ byte-for-byte trong
`SceneLoadResult`; serialization ra storage vẫn là trách nhiệm codec/package.

Snapshot phải ghi revision nhất quán tại safe boundary; không trộn state từ hai
step hoặc hai package configuration.

## 5. Portability

Project không lưu OS absolute path, process handle, GPU handle, URL tạm hoặc pointer.
Package namespace dùng virtual path tương đối. Backend archive/directory/memory/
remote stream là adapter ngoài format semantic.
