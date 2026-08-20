# ifol-engine — sơ đồ kiến trúc và luồng hoạt động

## 1. Một câu trả lời ngắn

`ifol-engine` là headless composition runtime: nhận package, config runtime và
provider từ bên ngoài; đăng ký chúng vào `ifol-ecs`; sau đó cung cấp lifecycle,
scene session, `step`, reconfigure và shutdown.

Nó không phải application, project editor, asset manager, renderer hoặc event
loop.

## 2. Vị trí trong hệ thống

```mermaid
flowchart LR
    Host[Host / CLI / Test runner\nloop + platform] --> Project[ifol-project\nmanifest + storage]
    Project --> Config[EngineConfig\nin-memory]
    Host --> Packages[Package providers\nEnginePackage]
    Packages --> Engine[ifol-engine\ncomposition runtime]
    Config --> Engine
    Engine --> ECS[ifol-ecs\nworld + schedule + execution]
    Feature[feature packages\nname / transform / shape / gpu] --> Packages
    Engine --> Reports[StepReport / diagnostics]
    Reports --> Host
```

## 3. Engine sở hữu gì

```text
EngineBuilder
├── package candidates
├── EngineConfig
├── provider candidates
└── registration candidates

EngineRuntime
├── ifol_ecs::EcsRuntime
├── resolved PackageLock
├── CommandRegistry
├── ProviderManager
├── SchemaRegistry / MigrationRegistry
├── NamespaceRegistry
├── active SceneId + scene entity ownership
├── EngineState
└── monotonic revision
```

Project storage, asset files, GPU device, window, clock, thread pool và loop
không nằm trong runtime.

## 4. Dữ liệu đầu vào và đầu ra

| Input | Ai tạo | Engine làm gì | Output |
|---|---|---|---|
| `EngineConfig` | host/project adapter | chọn package roots, kiểm tra expected lock, nhận namespace snapshot | resolved runtime composition |
| `EnginePackage` | feature/plugin bên ngoài | resolve dependency rồi gọi `register` theo thứ tự deterministic | staged ECS/provider/schema contribution |
| `ResourceProvider` | package/host | init theo dependency DAG, teardown ngược | root resource/service trong ECS |
| `SceneDocument` + `SceneId` | host/project/codec | validate, decode/migrate, load-new-before-replace | `SceneLoadResult`, active scene |
| `StepInput` | host loop | chạy ECS đúng một lần | `StepReport`, revision mới |
| `ReconfigurationRequest` | host/package coordinator | compile candidate rồi swap atomic | `ReconfigurationReport` |
| shutdown request | host | teardown provider + ECS | `ShutdownReport` |

Engine không nhận trực tiếp TOML, đường dẫn file, keyboard event, window event,
asset path hay GPU command.

## 5. Luồng build

```mermaid
sequenceDiagram
    participant H as Host
    participant C as EngineConfig
    participant B as EngineBuilder
    participant R as Resolver
    participant P as Packages
    participant T as RegistrationTransaction
    participant E as ECS
    participant V as Providers

    H->>C: tạo config in-memory
    H->>B: register packages/providers + with_config
    B->>R: resolve roots + dependencies
    R-->>B: deterministic PackageLock
    B->>P: register(context) theo lock
    P-->>T: staged components/systems/providers/schemas
    T->>E: commit + compile schedule
    E-->>T: compiled ECS runtime
    T->>V: init theo DAG
    V-->>B: ready hoặc typed failure + rollback
    B-->>H: EngineRuntime Ready
```

## 6. Luồng một step

```mermaid
stateDiagram-v2
    [*] --> Ready
    Ready --> Stepping: step(StepInput)
    Stepping --> Ready: run_once thành công
    Stepping --> Faulted: ECS/provider invariant failure
    Ready --> Shutdown: shutdown()
    Faulted --> Shutdown: shutdown()
    Shutdown --> [*]
```

```text
step(input)
  -> kiểm tra Ready
  -> chuyển Stepping, chặn reentrancy
  -> EcsRuntime::run_once()
  -> tăng revision nếu thành công
  -> chuyển Ready
  -> trả StepReport
```

Engine không tự lặp, không sleep, không pacing và không tự gọi step lần hai.

## 7. Luồng scene

```mermaid
flowchart TD
    D[SceneDocument] --> Validate[validate keys + records]
    Validate --> Decode[codec package decode]
    Decode --> Migrate[migration chain nếu cần]
    Migrate --> Spawn[spawn entities + attach components]
    Spawn --> Publish[publish active SceneId]
    Publish --> Replace[despawn scene cũ]
    Decode -->|error| Abort[giữ nguyên scene cũ]
    Migrate -->|error| Abort
    Spawn -->|error| Rollback[rollback partial load]
    Rollback --> Abort
```

`WORLD_ENTITY` và world singleton resources không thuộc ownership của active
scene nên không bị `clear_scene` xóa.

## 8. Luồng reconfigure

```text
live runtime
  -> caller chuẩn bị candidate transaction/registries
  -> tạo ECS staging runtime
  -> commit registration + compile
  -> init candidate providers
  -> teardown provider cũ
  -> swap ECS/registries/lock cùng một boundary
  -> revision++ và trả report
```

Nếu staging thất bại, live runtime không đổi. Nếu teardown side effect thất bại,
engine chuyển `Faulted` vì không thể rollback external side effect.

## 9. Những thứ phải nằm bên ngoài

```text
ifol-project : project files, save/load, storage, lock syntax
ifol-asset   : asset identity, import, decoder, cache policy
ifol-gpu     : device, resources, render graph, pipeline, execution
feature pkg  : name, hierarchy, transform, shape, animation semantics
host         : loop, window, input, timing, platform, process lifecycle
```

Các phần trên chỉ tương tác với engine qua package/provider/codec/transaction
contract; engine không hard-code domain enum để biết chúng là gì.
