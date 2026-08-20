# Public API và typed diagnostics

## 1. API surface

Các mục dưới đây là contract đang được expose. Hiện tại đã có `state`, `revision`,
`package_lock`, `schema_registry`, `migration_registry`,
`load_scene`, `step`, `reconfigure` và `shutdown`; các mục còn lại là phần mở
rộng tiếp theo, không được giả định là đã có trong crate.

```text
EngineBuilder
├── register_package(package)
├── with_provider(provider)
├── with_config(config)
└── build() -> EngineRuntime

EngineRuntime
├── state()
├── active_packages()
├── schema_registry()
├── migration_registry()
├── namespace_registry()
├── load_scene(document)
├── load_scene_as(scene_id, document)
├── active_scene()
├── clear_scene()
├── step(input) -> StepReport
├── reconfigure(candidate_request) -> ReconfigurationReport
└── shutdown() -> ShutdownReport
```

`register_package` nhận một `EnginePackage`, không nhận domain enum hoặc path
implementation. Runtime lưu `PackageLock` immutable của lần build để host và
diagnostics có thể kiểm tra chính xác package set đã được resolve.
`load_scene` và `load_scene_as` là boundary để nạp `SceneDocument` vào ECS;
document được validate,
decode/migrate theo schema do package đăng ký, rollback khi lỗi, và chỉ tăng
revision khi thành công. `load_scene_as` thay active scene theo chính sách
load-new-before-replace; `clear_scene` chỉ xóa entity của scene, giữ world
singleton và package registration.
`namespace_registry` là snapshot các claim đã được package commit. Engine không
ghi snapshot này ngược vào project storage; host/project layer tự quyết định
cách lưu nếu cần.
Reconfigure cũng nhận các registry ứng viên đã được chuẩn bị đầy đủ; engine chỉ
swap ECS, command, schema, migration và package lock sau khi compile thành công.

`ReconfigurationRequest` là candidate transaction boundary: caller chuẩn bị
registry/package candidate, engine validate/commit/swap nguyên tử theo safe
boundary. API không mở raw
mutable `EcsRuntime` cho adapter bình thường. Bootstrap/test có controlled access
riêng; package chỉ dùng `RegistrationContext` và `SystemContext`.

Command/query/event là mechanism generic; concrete IDs, payload validation và
handler do package đăng ký. Engine không có enum chứa AddShape/LoadAsset hoặc
history stack mặc định. Command mutation chỉ commit tại transaction/safe boundary.

## 2. StepInput và StepReport

`StepInput` hiện chứa correlation envelope generic. Typed package input được đưa
qua provider/component contract; engine không hard-code keyboard, timeline hay
render request.

`StepReport` tối thiểu chứa:

- engine/project/package revisions;
- ECS `RunReport`;
- package/service diagnostics đã publish;
- committed changes/events ở step boundary;
- typed warning/error summary;
- timing chỉ để quan sát, không tham gia semantic equality.

## 3. Error taxonomy

```text
EngineError
├── InvalidState
├── PackageDiscovery/Resolution/Dependency
├── CapabilityUnavailable
├── Registration/Namespace/Schema
├── ResourceInitialization
├── SceneLoad/Migration
├── EcsCompile/EcsExecution
├── ReconfigureRollback
└── Shutdown
```

Error phải chứa package/schema/scene/resource identity và operation context khi
có. Không parse error bằng string; message chỉ dành cho người đọc.

## 4. Panic và cancellation policy

Invalid input, missing package, malformed project, provider failure, stale state
và lifecycle misuse trả typed error. Panic chỉ dành cho bug invariant nội bộ đã
được chứng minh.

Job dài thuộc package/service; engine chỉ giữ handle/state/cancel contract. `step`
không chờ I/O không giới hạn. Shutdown có deadline/policy do host truyền, không
tự sleep hoặc spin.
