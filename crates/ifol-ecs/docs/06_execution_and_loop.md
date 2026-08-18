# Execute, schedule và outer loop

## 1. Một execution pass

ifol-ecs cung cấp một pass, không cung cấp realtime loop.

~~~mermaid
flowchart TD
    Start["run_once"] --> Activate["Evaluate activation cache/conditions"]
    Activate --> Phase["Traverse compiled PhaseGraph"]
    Phase --> System["Run bound systems"]
    System --> Commands["Flush deferred commands at safe point"]
    Commands --> Revision["Commit revisions/cache invalidation"]
    Revision --> Report["RunReport"]
~~~

API mục tiêu:

~~~rust
let report = runtime.run_once()?;
~~~

## 2. Outer loop thuộc host

~~~mermaid
flowchart LR
    HostLoop["Engine/CLI/export loop"] --> Prepare["Host updates external state"]
    Prepare --> ECS["EcsRuntime::run_once"]
    ECS --> Output["Host reads report/output"]
    Output --> HostLoop
~~~

Host quyết định một pass là frame, export sample, simulation step hay test
iteration. ECS không poll input, sleep, present surface hoặc encode file.

Safe point áp dụng command theo declaration order. Spawn có thể trả ticket để
insert/remove/despawn trong cùng buffer; command target lỗi được trả về dưới
dạng `EcsError`, không bị nuốt. Command còn lại sau lỗi bị bỏ. Nếu system tự
trả `SystemError`, các command nó đã xếp cũng bị bỏ và không làm thay đổi World.

`ExecutionPolicy` là cấu hình của runtime, không phải kiến thức của system:
`CollectErrors` ghi lỗi vào `RunReport` và chạy tiếp, `StopPhaseOnError` bỏ phần
còn lại của phase hiện tại, còn `FailFast` trả `EcsError::SystemExecutionFailed`
ngay cho host.

## 3. Execute preconditions

Trước run, ECS phải có compiled schedule hợp lệ. Nếu registration/graph thay đổi,
runtime yêu cầu compile lại hoặc tự compile theo policy đã chọn. Không chạy plan
stale mà không có diagnostic.

## 4. Determinism

Cùng World state, registration revision, graph revision và policy phải tạo cùng
logical execution order. Timing/profiling là metadata.
