# Test và acceptance plan

## 1. Test pyramid

```text
Unit
  identity · parser · resolver · graph · state machine · path validation
Integration
  package -> registration -> ECS compile -> step -> report
Contract
  public API từ external crate + package author surface
Adversarial
  malformed input · collision · rollback · stale state · reentrancy
Property
  ordering permutations · DAG generation · round-trip records
Stress
  many packages/scenes/records/reconfigure cycles
Portability
  native + no-default-features + wasm compile where supported
```

## 2. Bắt buộc theo module

### Lifecycle

- empty build, empty compile và empty step;
- step trước Ready, sau Shutdown, reentrant/concurrent step;
- repeated shutdown/unload;
- execution counter/revision wrap policy;
- panic-free invalid state handling.

### Package resolver

- duplicate ID/version/fingerprint;
- dependency missing, version conflict, self-cycle và multi-node cycle;
- deterministic winner/order bất kể discovery order;
- unsupported engine API/platform/capability;
- lock mismatch và package substitution.

### Registration transaction

- failure tại từng contribution type;
- duplicate component/resource/system/phase/schema/namespace;
- foreign component/system ID;
- invalid access descriptor và phase cycle;
- provider failure trước/giữa/sau dependency chain;
- không leak/drop hai lần và không partial activation.

### Project/container

- empty project và zero scene;
- malformed/truncated/oversized manifest theo configured limits;
- path traversal, absolute path, reserved/duplicate namespace;
- unknown package data preservation;
- deterministic canonical save;
- interrupted save không phá bản hợp lệ trước đó ở backend hỗ trợ transaction.

### Scene/schema

- zero entity/component;
- duplicate serialized entity key/component record;
- entity remap và stale reference;
- schema version cũ/mới/unknown;
- migration chain, gap, cycle và failure rollback;
- codec reject payload;
- opaque byte-preserving round-trip;
- load failure không thay đổi current runtime.

### Resource lifecycle

- one instance trên `WORLD_ENTITY`;
- owned/bound/derived provider;
- missing binding/dependency/cycle;
- init/drop exactly once;
- reverse teardown order;
- clear scene không xóa persistent runtime resource;
- unload/reconfigure/shutdown theo declared lifetime policy.

### Step và determinism

- một call tương ứng đúng một ECS pass;
- no hidden second pass/retry/sleep;
- same project/lock/input tạo cùng registration và execution order;
- queued change chỉ commit ở safe boundary;
- service pending/ready/failed không block vô hạn;
- report revisions và diagnostics chính xác.

### Command/query/event boundary

- unknown/duplicate ID và unsupported version;
- malformed payload, stale precondition và unauthorized capability context;
- handler failure rollback, event chỉ phát sau commit;
- command order/correlation/transaction revision chính xác;
- query không mutate runtime;
- package mới đăng ký command mà không sửa enum/source engine.

## 3. Dev-only test packages

Ít nhất ba package fixture:

```text
pkg-alpha: component + resource + system + schema + namespace
pkg-beta: phụ thuộc alpha và đọc resource alpha
pkg-fail: có thể fail theo từng registration/provider/migration stage
```

Fixture không được ghi artifact vào source tree. Filesystem test dùng temporary
directory; correctness không phụ thuộc timing hoặc network.

## 4. Verification gates

```text
cargo fmt --package ifol-engine -- --check
cargo check -p ifol-engine --all-targets
cargo clippy -p ifol-engine --all-targets -- -D warnings
cargo test -p ifol-engine --all-targets
cargo test -p ifol-engine --doc
cargo check -p ifol-engine --no-default-features --all-targets
cargo check -p ifol-engine --target wasm32-unknown-unknown
```

WASM gate chỉ được đánh dấu supported khi dependency set thực tế compile; không
ẩn unsupported implementation bằng test skip không có lý do.

## 5. Definition of Done

- mọi test group trên có executable test, không chỉ report Markdown;
- không ignored test để né edge case bắt buộc;
- public API có external integration tests;
- lỗi registration/load/reconfigure chứng minh rollback;
- project/package order và save output deterministic;
- engine không import feature/subsystem production cụ thể;
- engine không có platform loop hoặc UI/editor logic;
- test package mới được thêm mà không sửa production engine source;
- docs khớp API và mọi lệnh verification xanh.
