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
