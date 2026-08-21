# Ownership và lifecycle

## 1. EngineRuntime

`EngineRuntime` là một session headless hữu hạn, sở hữu một `EcsRuntime` và
metadata composition cần để load/reconfigure/snapshot.

```text
EngineRuntime
├── EcsRuntime
├── ActivePackageSet
├── SchemaRegistry
├── NamespaceRegistry
├── EngineRevision
└── Diagnostics
```

Registry nghiệp vụ không được nhân đôi dữ liệu ECS. `ActivePackageSet` chỉ giữ
identity/version/lifecycle package; component/system/phase thực tế thuộc ECS.

`ifol-engine` là library crate phụ thuộc `ifol-ecs` bằng public Rust API. Engine
không phụ thuộc trực tiếp `ifol-gpu`, `ifol-asset` hoặc package production. Test
fixtures là dev-dependencies và không được rò vào public API.

`EngineBuilder::with_config` nhận `EngineConfig` in-memory. Project manifest,
storage và lock-file được `ifol-project` chuyển đổi thành config trước khi gọi
engine. Runtime không giữ project container, filesystem hay persistence state.

## 2. State machine

```mermaid
stateDiagram-v2
    [*] --> Building
    Building --> Ready: validate + atomic commit + compile
    Building --> Failed: typed build error
    Ready --> Stepping: step(input)
    Stepping --> Ready: StepReport
    Stepping --> Faulted: fail-fast invariant/service failure
    Ready --> Ready: stage/reconfigure candidate
    Ready --> Faulted: provider teardown failure
    Ready --> Shutdown
    Faulted --> Shutdown
    Shutdown --> [*]
```

Một method chỉ hợp lệ ở state đã định nghĩa; lời gọi sai state trả typed error,
không panic và không âm thầm no-op.

## 3. Loop boundary

Engine không sở hữu `while`, `requestAnimationFrame`, window event loop, sleep,
frame pacing hoặc worker queue.

```rust
while let Some(input) = host.next_input()? {
    let report = engine.step(input)?;
    host.consume(report)?;
}
```

Engine sở hữu semantics của một step:

1. kiểm tra runtime đang ở `Ready`;
2. chuyển sang `Stepping` để chặn reentrancy;
3. gọi `EcsRuntime::run_once()` đúng một lần;
4. tăng engine revision khi pass thành công;
5. chuyển lại `Ready` và trả `StepReport`.

Không tự retry, sleep hoặc bắt đầu step thứ hai. Reentrancy và concurrent mutable
step bị từ chối.

## 4. Resource root

Service/capability dùng chung được expose bằng component trên `WORLD_ENTITY`:

```text
register_provider(provider)
  -> register_world_singleton<T>()
  -> provider.create(context)
  -> insert_world_component<T>(value)
```

Provider có thể tạo pure state hoặc typed handle tới subsystem. Provider failure
phải rollback toàn registration batch. Shutdown gọi cleanup theo thứ tự dependency
ngược; không dựa vào global mutable static.

## 5. Clear, unload và shutdown

- `clear_scene`: xóa entity scene, giữ root resources/package registrations;
- `clear_scene`: đóng active scene và giữ package/runtime resources;
- project container lifecycle thuộc host/session boundary; engine không đọc,
  ghi hoặc unload filesystem;
- `reconfigure`: chuẩn bị một runtime ECS mới và publish khi compile thành công;
  ECS, command/schema/migration registry và package lock được swap cùng một
  commit, lỗi staging giữ nguyên runtime cũ; lỗi teardown provider chuyển runtime
  sang `Faulted` vì external side effect không rollback. Đây là composition
  replacement, chưa phải state migration: entity/component state của ECS cũ
  không được tự động giữ lại;
- `shutdown`: ngăn step mới, drain/cancel job theo policy, drop root resources,
  shutdown ECS và trả report.

Mỗi operation phải idempotent hoặc trả typed state error rõ ràng.
