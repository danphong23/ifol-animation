# System model và SystemContext

## 1. System là gì?

System là một executable unit nhận context được ECS kiểm soát. Implementation có thể
nằm trong feature crate, nhưng instance được lưu trong SystemRegistry của
EcsRuntime.

~~~mermaid
flowchart LR
    Impl["Feature system implementation"] --> Register["register_system"]
    Register --> Registry["ECS SystemRegistry"]
    Registry --> Binding["Phase binding"]
    Binding --> Execute["SystemContext + run"]
~~~

## 2. System không biết phase hoặc system khác

~~~rust
trait System {
    fn run(&mut self, ctx: &mut SystemContext<'_>) -> Result<(), SystemError>;
}
~~~

System chỉ biết access/query được context cấp, command buffer và execution metadata.
System không tự gọi phase khác hoặc giữ reference tới system khác.

## 3. Registration metadata

~~~text
SystemRegistration
├── SystemId
├── implementation
├── AccessDescriptor
├── RunCondition list
└── debug metadata
~~~

Phase binding nằm ở PhaseNode.system_bindings, không nằm trong system logic.

## 4. Context boundary

~~~text
SystemContext
├── read/query access
├── mutable tracked query access
├── WorldRef<T>
├── deferred Commands
├── execution id/revision
└── diagnostics marker
~~~

Không expose raw &mut World cho system thông thường. Điều này cho phép ECS kiểm
soát aliasing, change tracking, structural mutation và future parallelism.

## 5. Error

System error là structured data. Scheduler quyết định fail-fast, stop-phase hoặc
collect-and-continue theo runtime policy.
