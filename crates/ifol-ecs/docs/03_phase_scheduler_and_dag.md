# Phase registry, phase graph và system binding

## 1. Phase là gì?

Phase là một node trong execution graph. Nó không phải frame, clock, input stage
hay semantic cố định. Phase chỉ định một nhóm system được chạy trong cùng vùng thứ
tự.

Feature tạo phase bằng `PhaseId::new(name)`. Core không có phase mặc định và không
biết input, animation, render hay GPU.

~~~text
PhaseNode
├── PhaseId
├── label/debug metadata
├── system bindings
├── before/after edges
└── phase-local execution policy
~~~

## 2. Ai sở hữu phase graph?

Bên ngoài gọi API đăng ký, nhưng ifol-ecs lưu và sở hữu graph.

~~~mermaid
flowchart LR
    Feature["Feature registration"] --> API["EcsRuntime registration API"]
    API --> Registry["PhaseRegistry + SystemRegistry"]
    Registry --> Graph["ECS-owned PhaseGraph"]
    Graph --> Compile["ECS-owned compiler"]
    Compile --> Plan["CompiledSchedule"]
~~~

Feature không giữ graph khác để tự chạy. Sau registration, runtime ECS là source
of truth.

## 3. System không biết phase

~~~mermaid
flowchart TB
    Impl["System implementation<br/>logic + context access"] --> SR["SystemRegistry"]
    Phase["PhaseNode: animation.evaluate"] --> Binding["SystemBinding"]
    SR --> Binding
    Binding --> Plan["Compiled phase plan"]
~~~

System registration và phase binding là hai việc khác nhau:

~~~rust
let id = runtime.register_system(
    "animation.evaluate.system",
    AnimationSystem::new(),
    access_descriptor,
    vec![],
)?;
let phase = PhaseId::new("animation.evaluate");
runtime.register_phase(phase.clone())?;
runtime.attach_system(&phase, id)?;
~~~

System không chứa phase và không gọi system khác.

## 4. Access descriptor

Mỗi system khai báo component types đọc/ghi. Schedule lưu metadata để kiểm tra
aliasing, chuẩn bị cho parallelism và cấp đúng quyền qua SystemContext. Executor
hiện chạy tuần tự deterministic; access contract không được bỏ vì nó là nền tảng
cho các executor khác về sau.

## 5. Graph và thứ tự

~~~mermaid
flowchart LR
    A["animation.evaluate<br/>AnimationSystem<br/>CurveSystem"]
    B["hierarchy.resolve<br/>HierarchySystem"]
    C["render.prepare<br/>RenderCacheSystem"]
    D["render.build_graph<br/>GraphBuildSystem"]
    A --> B --> C --> D
~~~

System trong một phase có declaration order deterministic. Core hiện chỉ có
dependency giữa phase; system-level edges hoặc parallel batch là extension phải
được thiết kế thêm, không phải hành vi ngầm.

## 6. Run conditions

~~~rust
enum RunCondition {
    Always,
    WorldHas(ComponentId, &'static str),
    All(Vec<RunCondition>),
    Any(Vec<RunCondition>),
}
~~~

WorldHas(T) nghĩa là WORLD_ENTITY có component T. Type phải được đăng ký khi build
schedule; vắng instance là trạng thái runtime và dẫn tới skip.

Typed adapter có thể suy ra:

| Parameter | Access | Condition |
|---|---|---|
| `ctx.world_ref::<T>()?` | Read T trên root | WorldHas(T) |
| `Option<T>` do system tự xử lý | Read T trên root | Không có |
| Query<Q> | Access theo Q | Không có |

## 7. Compile validation

~~~mermaid
flowchart TD
    Input["Registrations + bindings"] --> IDs["Validate IDs and references"]
    IDs --> Edges["Build phase edges"]
    Edges --> Cycle["Detect cycle"]
    Cycle --> Order["Deterministic topological order"]
    Order --> Bind["Resolve system bindings"]
    Bind --> Plan["CompiledSchedule owned by ECS"]
~~~

Logical graph và compiled plan phải tách nhau. Recompile không được làm mất World
data; revision tăng để cache cũ bị invalidated.

## 8. Run semantics

Một lần run đi qua phase/system theo compiled order, đánh giá conditions, chạy
system đủ điều kiện, flush deferred commands, rồi trả RunReport.

## 9. Error và panic policy

System error phải có typed diagnostic. Host chọn fail-fast, stop-phase hoặc
collect-and-continue. Input invalid, cycle và missing dependency không được panic.

## 10. Parallelism

Execution order deterministic là contract. Parallel executor chỉ được dùng khi
access descriptor, command buffer, safe point và deterministic ordering đã được
kiểm chứng.
