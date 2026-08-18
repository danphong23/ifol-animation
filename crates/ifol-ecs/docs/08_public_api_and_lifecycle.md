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
│   ├── validate
│   ├── compile
│   └── compilation_revision
├── execution
│   ├── run_once
│   ├── run_once_with_options
│   └── report/diagnostics
└── lifecycle
    ├── clear
    ├── reconfigure
    └── shutdown
~~~

## 3. Transactional registration

~~~mermaid
flowchart LR
    Batch["Feature registration batch"] --> Validate["Validate IDs/deps/access"]
    Validate -->|"fail"| Reject["No partial activation"]
    Validate -->|"pass"| Commit["Commit registries + graph revision"]
    Commit --> Compile["Compile owned schedule"]
~~~

Feature không active một nửa nếu dependency hoặc validation thất bại.

## 4. Read/mutate boundary

UI, MCP và CLI dùng Engine Command/Query boundary. Engine gọi public ECS API.
Feature systems dùng SystemContext. Host bootstrap dùng runtime API theo lifecycle
contract.
