# Public API và lifecycle

## 1. Runtime lifecycle

~~~mermaid
flowchart TD
    New["EcsRuntime::new"] --> Register["Register components/phases/systems"]
    Register --> Bind["Attach bindings + phase edges"]
    Bind --> Compile["validate + compile"]
    Compile --> Use["read/mutate World through API"]
    Use --> Run["run_once"]
    Run --> Reconfigure["optional registration change"]
    Reconfigure --> Compile
    Run --> Clear["clear/shutdown"]
~~~

## 2. API groups

~~~text
EcsRuntime
├── registration
│   ├── register_component<T>
│   ├── register_phase
│   ├── register_system
│   ├── attach_system
│   └── add_phase_edge
├── world access
│   ├── spawn/despawn
│   ├── insert/get/get_mut/remove
│   ├── insert_world_component
│   └── query
    ├── compilation
    │   ├── compile
    │   └── phase graph revision check
    ├── execution
    │   ├── run_once
    │   ├── execution policy
    │   └── report/diagnostics
└── lifecycle
    ├── clear
    ├── reconfigure
    └── shutdown
~~~

## 3. Registration and compilation boundary

~~~mermaid
flowchart LR
    Batch["Host-prepared feature contributions"] --> Validate["Validate IDs/deps/access"]
    Validate -->|"fail"| Reject["No partial activation"]
    Validate -->|"pass"| Commit["Commit registries + graph revision"]
    Commit --> Compile["Compile owned schedule"]
~~~

Mỗi lệnh registration tự kiểm tra lỗi cục bộ; `compile()` kiểm tra toàn bộ
component access, conditions, bindings và phase graph trước khi publish schedule.
Nếu compile thất bại, schedule cũ vẫn bị coi là stale và host phải sửa cấu hình
hoặc tạo lại compilation; World data không bị thay đổi.

## 4. Read/mutate boundary

UI, MCP và CLI dùng Engine Command/Query boundary. Engine gọi public ECS API.
Feature systems dùng SystemContext. Host bootstrap dùng runtime API theo lifecycle
contract.
