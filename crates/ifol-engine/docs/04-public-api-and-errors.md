# Public API và typed diagnostics

## 1. API surface mục tiêu

```text
EngineBuilder
├── register_package(package)
├── add_package_source(source)
├── bind_host_resource<T>(provider)
├── with_project(project)
├── open_project(source, policy)
└── build() -> EngineRuntime

EngineRuntime
├── state()
├── active_packages()
├── project_info()
├── submit(command) -> CommandReceipt
├── query(request) -> QueryResult
├── step(input) -> StepReport
├── snapshot(request) -> Snapshot
├── reconfigure(change) -> ReconfigureReport
├── unload_project()
└── shutdown() -> ShutdownReport
```

`register_package` nhận một `EnginePackage`, không nhận domain enum hoặc path
implementation. Runtime lưu `PackageLock` immutable của lần build để host và
diagnostics có thể kiểm tra chính xác package set đã được resolve.

API cụ thể có thể tách type theo borrow/async requirement, nhưng không mở raw
mutable `EcsRuntime` cho adapter bình thường. Bootstrap/test có controlled access
riêng; package chỉ dùng `RegistrationContext` và `SystemContext`.

Command/query/event là mechanism generic; concrete IDs, payload validation và
handler do package đăng ký. Engine không có enum chứa AddShape/LoadAsset hoặc
history stack mặc định. Command mutation chỉ commit tại transaction/safe boundary.

## 2. StepInput và StepReport

`StepInput` chỉ chứa envelope generic: correlation/revision và typed package input
đã đăng ký. Engine không hard-code keyboard, timeline hay render request.

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
├── ProjectFormat/SceneLoad/Migration
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
