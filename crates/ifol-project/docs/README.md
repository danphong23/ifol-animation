# ifol-project

`ifol-project` là tầng host/persistence đứng trên `ifol-engine`.

Nó chịu trách nhiệm cho manifest project, package lock file và storage backend.
Nó không chạy ECS, không giữ loop, không biết render/asset/animation và không
đăng ký feature cụ thể. `ProjectContainer::engine_config()` dịch dữ liệu đã đọc
thành `ifol_engine::EngineConfig`; engine nhận config in-memory và chạy độc lập
với filesystem.

```text
project files / storage -> ifol-project -> EngineConfig -> ifol-engine -> ifol-ecs
```

## Public contract

```text
storage.rs   -> ProjectStorage, MemoryStorage, path validation
manifest.rs  -> ProjectManifest, package requirements, format version
lockfile.rs  -> PackageLockFile <-> ifol_engine::PackageLock
container.rs -> atomic save/load and EngineConfig translation
```

Public API vẫn được re-export tại crate root; cấu trúc module nội bộ không buộc
host đoán project được lưu bằng backend nào. `ProjectStorage::write_files` là
batch boundary: backend production phải publish toàn bộ batch hoặc trả lỗi mà
không để project ở trạng thái nửa ghi.

## Verification

```text
cargo test -p ifol-project --all-targets
cargo clippy -p ifol-project --all-targets -- -D warnings
cargo doc -p ifol-project --no-deps
```

Bộ acceptance nằm tại `tests/project_acceptance.rs`; báo cáo đọc được nằm trong
`tests/reports/`. Chúng kiểm tra project-to-engine bootstrap, package lock,
package execution, scene lifecycle, reconfiguration boundary và shutdown.
