# ifol-engine Architecture Manual

Đây là architecture và acceptance manual hiện hành của `ifol-engine`. Engine là
headless composition runtime quanh `ifol-ecs`, không phải application shell và
không chứa feature nghiệp vụ. `EngineConfig` là boundary runtime thuần in-memory;
persistence project thuộc `ifol-project`.

## Thứ tự đọc

1. [01-ownership-and-lifecycle.md](01-ownership-and-lifecycle.md)
2. [02-package-and-registration.md](02-package-and-registration.md)
3. [03-project-scene-and-namespace.md](03-project-scene-and-namespace.md)
4. [04-public-api-and-errors.md](04-public-api-and-errors.md)
5. [05-implementation-plan.md](05-implementation-plan.md)
6. [06-test-and-acceptance-plan.md](06-test-and-acceptance-plan.md)
7. [07-current-status.md](07-current-status.md)

## Tóm tắt bất biến

```text
Host owns loop and project persistence
Engine owns composition/session
ECS owns runtime state/schedule/execution
Package owns feature semantics and project namespace
Subsystem owns specialized implementation
```

Không có package thì engine vẫn chạy rỗng hợp lệ. Mọi chức năng xuất hiện thông
qua registration, không qua enum/list hard-code trong engine.

Manual nằm cùng crate để thay đổi public contract, implementation và acceptance
tests luôn được review trong cùng phạm vi. Synthetic package fixtures chỉ tồn tại
trong integration tests; chúng không phải production API của crate.
