# Registry và Public API

## 1. Registry nằm trong EcsRuntime

Registry là nơi ECS giữ identity/runtime metadata, không phải map do feature tự
quản lý.

~~~mermaid
flowchart TB
    Runtime["EcsRuntime"] --> CR["ComponentRegistry"]
    Runtime --> SR["SystemRegistry"]
    Runtime --> PR["PhaseRegistry"]
    CR --> World["World storage"]
    SR --> Graph["Phase graph bindings"]
    PR --> Graph
~~~

Các registry tối thiểu:

| Registry | Giữ gì | Dùng cho |
|---|---|---|
| ComponentRegistry | TypeId, ComponentId, metadata | storage/query/access |
| SystemRegistry | SystemId, implementation, access/conditions | binding/execute |
| PhaseRegistry | PhaseId, descriptor, edges | graph/compile |

## 2. Registration API

~~~rust
let position = runtime.register_component::<Position>()?;
let phase = PhaseId::new("movement");
runtime.register_phase(phase.clone())?;
let system = runtime.register_system(
    "movement",
    MovementSystem,
    access_descriptor,
    vec![],
)?;

runtime.attach_system(&phase, system)?;
runtime.add_phase_edge(&phase_a, &phase_b)?;
runtime.compile()?;
~~~

Các hàm registration chỉ thay đổi logical model và mỗi lệnh lỗi không commit
trạng thái một phần. `compile()` là transaction boundary của execution schedule:
chỉ schedule hợp lệ mới được publish để chạy.

## 3. World data API

~~~text
runtime.spawn()
runtime.despawn(entity)
runtime.insert(entity, component)
runtime.get<T>(entity)
runtime.get_mut<T>(entity)
runtime.remove<T>(entity)
runtime.insert_world_component(value)
runtime.query<Q>()
runtime.query_mut<Q>()
~~~

UI/MCP/CLI thường không gọi trực tiếp các hàm này mà đi qua Engine Command/Query
boundary. Feature bootstrap và test có thể dùng chúng theo lifecycle contract.

## 4. Compile và revision

~~~mermaid
flowchart LR
    Mutation["registration mutation"] --> Revision["registration/graph revision++"]
    Revision --> Dirty["compiled plan stale"]
    Dirty --> Compile["validate + compile"]
    Compile --> Ready["runtime ready"]
~~~

Compile không tạo World mới và không làm mất component data.
