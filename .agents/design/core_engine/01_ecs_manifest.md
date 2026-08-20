# ifol-ecs: ECS Execution Substrate

Đây là contract kiến trúc cấp workspace của ifol-ecs. Chi tiết API và data model
nằm trong ECS architecture manual tại crates/ifol-ecs/docs/README.md.

## 1. Bản chất

ifol-ecs là một execution substrate hoàn chỉnh. Nó không phải callback runner tạm
thời và không phải animation/render engine. Core sở hữu:

- World, entity lifecycle và component storage;
- component/system/phase registries;
- phase graph và compiled schedule;
- query plan, change tracking, activation state và deferred commands;
- executor, lifecycle và RunReport.

Feature/Engine chỉ gọi public registration/data/execution API. Feature cung cấp
component type và system implementation; ECS giữ chúng trong runtime sau khi đăng
ký. ECS không biết semantic của component hoặc phase.

~~~mermaid
flowchart TB
    External["Feature · Engine · CLI · Test"] -->|"public API: register / mutate / read / run"| Runtime
    subgraph Runtime["ifol-ecs::EcsRuntime sở hữu"]
        Registries["Component · System · Phase registries"]
        Graph["Phase graph + system bindings"]
        Plan["Compiled schedule + query plans"]
        World["World + WORLD_ENTITY + component storage"]
        Cache["Revision/change/activation cache"]
        Executor["Executor + deferred command queues"]
        Report["RunReport + diagnostics"]
        Registries --> Graph --> Plan --> Executor
        World --> Executor
        Cache --> Plan
        Executor --> Report
    end
~~~

## 2. Ranh giới mù

Core không hard-code hoặc import:

~~~text
Time · Input · Scene · Transform · Hierarchy
Animation · Shape · Image · Video · Font
GPU · Asset · Decode · Encode · Project · UI · MCP
~~~

Một feature có thể đăng ký các tên trên như component, phase hoặc system; ECS chỉ
xử lý chúng như ID, descriptor và dữ liệu generic.

## 3. Registration và ownership

Bên ngoài gọi các API dạng:

~~~rust
runtime.register_component::<T>();
runtime.register_phase(phase_id, phase_descriptor);
runtime.register_system(system_id, system_registration);
runtime.attach_system(phase_id, system_id);
runtime.add_phase_edge(before, after);
runtime.compile()?;
~~~

Sau đó EcsRuntime sở hữu registry, graph binding và compiled plan. Bên ngoài không
tự giữ một phase graph song song để chạy.

## 4. Execution boundary

Core cung cấp một execution pass:

~~~rust
let report = runtime.run_once()?;
~~~

Outer loop thuộc Engine/CLI/exporter:

~~~text
host loop
  ├── cập nhật platform/service state qua public API
  ├── runtime.run_once()
  ├── đọc RunReport/output
  └── lặp theo policy realtime/export/test
~~~

ECS không sleep, poll window, đo FPS hay quyết định một pass có ý nghĩa là frame,
tick hay export step.

## 5. Invariant cấp core

- Input invalid bị từ chối bằng typed error trước khi compile/run.
- Không có phase/system dependency implicit ngoài contract đã đăng ký.
- Compiled plan deterministic với cùng registration/revision/capability.
- Cache chỉ là optimization; logical World/graph là source of truth.
- Mutation đi qua boundary có tracking; raw storage mutation không phải public API.
- WORLD_ENTITY sống suốt đời World và dùng cùng component/query storage.
- Một lần run luôn có report về executed/skipped/error/commands.
