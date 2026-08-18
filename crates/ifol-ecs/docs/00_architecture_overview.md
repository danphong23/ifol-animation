# Tổng quan kiến trúc ifol-ecs

Đây là đặc tả mục tiêu của execution substrate.

## ECS giống một work-graph engine

Tương tự ifol-gpu nhận graph/resource contract rồi validate, compile và execute,
ifol-ecs nhận registration/World mutation rồi validate, compile phase graph và
execute systems.

~~~mermaid
flowchart LR
    subgraph GPU["ifol-gpu"]
        GIn["Graph + resources"] --> GV["Validate"] --> GC["Compile flat plan"] --> GX["Execute"] --> GR["Execution report"]
    end
    subgraph ECS["ifol-ecs"]
        EIn["Registrations + World data"] --> EV["Validate"] --> EC["Compile schedule/query plans"] --> EX["Execute"] --> ER["RunReport"]
    end
~~~

## Bốn lớp bên trong EcsRuntime

~~~text
EcsRuntime
├── Registration layer
│   ├── ComponentRegistry
│   ├── SystemRegistry
│   └── PhaseRegistry
├── Logical model
│   ├── World
│   ├── PhaseGraph
│   └── SystemBindings
├── Compiled state
│   ├── CompiledPhasePlan
│   ├── QueryPlanCache (owned by World)
│   └── Revision metadata
└── Execution layer
    ├── SystemContext
    ├── Command queues
    ├── Safe-point flush
    └── RunReport
~~~

## Logical input và compiled output

~~~text
Logical input                         Compiled/runtime state

Component registrations       ─┐
System registrations           ├──> Component IDs + system table
Phase registrations            ┤
Phase edges + bindings         ├──> deterministic phase execution plan
World structural/data state   ─┘     query plan cache + revision metadata
~~~

Logical registration là source of truth. Compiled plan có thể bị invalidate và
build lại khi registration graph thay đổi; World data không được mất khi compile
lại.

## Core không tự đặt domain policy

Một profile có thể đăng ký:

~~~text
prepare → process → finalize
~~~

Profile khác có thể đăng ký:

~~~text
decode → inspect → export
~~~

Cả hai đều dùng cùng EcsRuntime. Không có phase mặc định trong core.
