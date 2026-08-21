# Current status và freeze boundary

## Trạng thái hiện tại

`ifol-engine` đã hoàn thành composition kernel nền tảng:

- `EngineBuilder` và lifecycle `Ready/Stepping/Faulted/ShuttingDown`;
- package identity, semantic version, dependency closure và lockfile;
- transactional registration của component, singleton, phase, system,
  command/query/event, schema, migration, provider và namespace;
- provider dependency DAG với rollback init và reverse teardown;
- `EngineConfig` in-memory cho required package closure, expected lock và namespace;
- persistence project đã có crate riêng `ifol-project` (manifest, lockfile,
  virtual storage và path containment); engine không compile hoặc sở hữu các
  kiểu persistence này;
- scene document generic, schema codec, migration và opaque preservation;
- scene session với `SceneId`, load-new-before-replace và `clear_scene`;
- atomic batch-write boundary cho project storage;
- dynamic reconfiguration qua một candidate transaction đã được caller chuẩn bị
  (composition replacement trên ECS runtime mới, chưa có state migration);
- một `step()` hữu hạn; host sở hữu loop, timing, window và platform events.

Engine không biết asset, render, shader, animation, input, game hay editor.
Những phần đó phải là package/provider/codec độc lập đăng ký từ bên ngoài.

## Giới hạn có chủ ý

- `ifol-project::ProjectContainer` quản lý manifest, lock và storage; nó không
  gán semantic cho thư mục `assets`, `render`, `animation` hoặc `game`.
- `EngineConfig` không đọc file và không giữ storage. `with_config` là API duy nhất
  để đưa runtime composition inputs vào engine.
- `EngineRuntime` quản lý một active scene session trên một ECS world. Scene mới
  được load hoàn tất trước, sau đó scene cũ mới bị remove; `WORLD_ENTITY` và
  singleton runtime không bị xóa bởi `clear_scene`.
- `ProjectStorage::write_files` là transaction boundary. Backend đơn giản có
  thể dùng default sequential implementation; backend cần crash safety phải
  override bằng commit nguyên tử.
- Reconfiguration nhận candidate registries/transaction đã chuẩn bị. Engine
  không tự đoán package loader, filesystem discovery hoặc domain policy.

## Verification hiện hành

```text
cargo fmt --package ifol-engine -- --check                 PASS
cargo check -p ifol-engine --all-targets                   PASS
cargo clippy -p ifol-engine --all-targets -- -D warnings   PASS
cargo test -p ifol-engine --all-targets -- --test-threads=1 PASS
cargo test -p ifol-engine --doc                            PASS
cargo check -p ifol-engine --no-default-features --all-targets PASS
```

Test suite engine bao gồm 34 unit tests và 88 integration tests ở slice 01–04,
06–10, 12–14. `ifol-project` có thêm 4 unit tests và 6 project acceptance
tests chạy xuyên boundary project -> engine; có rollback, deterministic resolver chain 256
package, scene replacement, malformed input, opaque record preservation và
project-to-engine headless bootstrap.

## Freeze rule

Có thể đóng `ifol-engine` khi các gate trên chạy trên worktree sạch, docs này
khớp source hiện hành, và không còn package/feature production cụ thể được thêm
vào engine core. Sau boundary này, mở rộng phải diễn ra bằng package độc lập như
`ifol-name`, `ifol-hierarchy`, `ifol-transform`, `ifol-shape`, `ifol-gpu` hoặc
provider tương ứng.

Project acceptance command:

```text
cargo test -p ifol-project --test project_acceptance
cargo doc -p ifol-engine -p ifol-project --no-deps
```
