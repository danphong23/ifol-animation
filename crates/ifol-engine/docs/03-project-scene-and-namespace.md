# Project, scene và package namespace

## 1. Phân chia trách nhiệm

```text
ifol-project (host/persistence)
├── project.toml
├── package.lock
├── storage backend
└── EngineConfig adapter

ifol-engine (runtime composition)
├── PackageLock
├── NamespaceRegistry
├── SchemaRegistry / MigrationRegistry
└── EngineRuntime + one ECS runtime
```

- `ifol-project` đọc/ghi manifest, lock và storage rồi tạo `EngineConfig`;
- `ifol-engine` không biết filesystem, project path, save/load hay package
  discovery;
- package tự định nghĩa namespace và semantics; engine chỉ validate collision.

Không crate nào mặc định tạo `assets`, `presets`, `render`, `animation`, `game`
hoặc `artifacts`. Package owner tự định nghĩa dữ liệu, cache policy và migration.

Khi host mở project, `required_packages` và lock được chuyển thành
`EngineConfig`. Package candidate ngoài transitive closure không được activate.
Engine chỉ so sánh `PackageLock` in-memory; nó không tự ghi ngược lock file.

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

Engine không thực hiện save. Nếu host cần save, package codec chuyển runtime component thành
versioned record. Unknown opaque record được giữ byte-for-byte trong
`SceneLoadResult`; serialization ra storage vẫn là trách nhiệm codec/package.

Snapshot phải ghi revision nhất quán tại safe boundary; không trộn state từ hai
step hoặc hai package configuration.

## 5. Portability

Project không lưu OS absolute path, process handle, GPU handle, URL tạm hoặc pointer.
Package namespace dùng virtual path tương đối. Backend archive/directory/memory/
remote stream là adapter ngoài format semantic.
