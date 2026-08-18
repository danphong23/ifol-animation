# ifol-ecs Architecture Manual

Đây là tài liệu chuẩn để hiểu và xây dựng ifol-ecs. Thứ tự đọc cũng là thứ tự sở
hữu dữ liệu trong runtime.

~~~mermaid
flowchart LR
    E["01 Entity"] --> W["02 World + Component"]
    W --> R["03 Registry + API"]
    R --> Q["04 Query"]
    R --> S["05 System"]
    S --> P["06 Phase Graph"]
    Q --> X["07 Execute"]
    P --> X
    W --> C["08 Cache + Change"]
    X --> L["09 Lifecycle"]
    L --> F["10 Feature registration"]
    F --> T["11 Test + acceptance"]
~~~

## Sơ đồ quyền sở hữu tổng thể

~~~mermaid
flowchart TB
    subgraph Caller["Bên ngoài: chỉ gọi API"]
        Feature["Feature package<br/>component types + system implementations"]
        Host["Engine / CLI / Test<br/>profile + commands"]
        Adapter["UI / MCP / Agent<br/>commands / queries"]
    end

    subgraph ECS["ifol-ecs: owner của runtime"]
        API["Public API boundary"]
        Registry["Registries"]
        World["World data"]
        Graph["Phase graph"]
        Plan["Compiled plans"]
        Cache["Caches + revisions"]
        Exec["Executor"]
        Report["RunReport"]
    end

    Feature --> API
    Host --> API
    Adapter --> Host
    API --> Registry
    API --> World
    Registry --> Graph --> Plan --> Exec
    World --> Exec
    Cache --> Plan
    Exec --> Report
~~~

## Runtime data flow

~~~mermaid
sequenceDiagram
    participant H as Host/Feature
    participant E as EcsRuntime
    participant W as World
    participant S as Schedule
    participant R as RunReport

    H->>E: register components/phases/systems
    H->>E: attach systems + phase edges
    E->>E: validate + compile owned plan
    H->>W: insert/read/mutate data through API
    H->>S: run_once()
    S->>W: query/mutate/queue commands
    S->>S: flush safe points + update revisions
    S-->>R: executed/skipped/errors/commands
    R-->>H: report
~~~

EcsRuntime là composition root bên trong crate. World, registry, graph, compiled
schedule và cache không được trở thành các map rời rạc do từng feature tự quản lý.

## Bản đồ tài liệu

| Chủ đề | Tài liệu |
|---|---|
| Entity, World, storage | 01_world_storage_and_query.md |
| World singleton/resource | 02_resources_and_data_flow.md |
| Registry và public API | 03_registry_and_api.md |
| Query và query plan | 04_query_and_plan.md |
| System và SystemContext | 05_system_model.md |
| Phase graph và binding | 03_phase_scheduler_and_dag.md |
| Execute và outer loop | 06_execution_and_loop.md |
| Cache và revision | 07_cache_and_revision.md |
| Lifecycle API | 08_public_api_and_lifecycle.md |
| Feature registration | 09_feature_registration_and_extension.md |
| Validation/report | 10_contracts_and_diagnostics.md |
| Test/acceptance | 04_test_feature_and_acceptance_plan.md, 11_test_and_acceptance_map.md |
