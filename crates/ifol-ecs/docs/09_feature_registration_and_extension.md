# Feature registration và extension

## 1. Feature là contributor, ECS là owner

~~~mermaid
flowchart LR
    F1["feature-animation"] -->|"registration batch"| ECS["ifol-ecs runtime"]
    F2["feature-render-core"] -->|"registration batch"| ECS
    F3["test-feature"] -->|"registration batch"| ECS
    ECS --> Registry["owned registries + phase graph"]
    ECS --> Run["owned executor"]
~~~

Feature có thể đóng góp component type, world singleton initializer/provider,
system implementation, phase node, phase edge, system binding, access/condition
metadata và debug labels.

Feature không tự tạo World, Schedule hoặc execution loop riêng cho cùng runtime.

## 2. Registration order

~~~mermaid
flowchart TD
    Discover["Discover feature packages"] --> Resolve["Resolve dependencies"]
    Resolve --> Batch["Create registration batch"]
    Batch --> ECSReg["EcsRuntime register"]
    ECSReg --> Validate["Validate all contributions"]
    Validate --> Compile["Compile ECS-owned plan"]
    Compile --> Load["Load World/project data"]
    Load --> Ready["Ready to run"]
~~~

Project/Engine chọn feature active, nhưng sau khi active runtime ECS vẫn là owner
của registration state.

## 3. Extension không phá core

Feature mới chỉ cần thêm component type, system implementation, phase binding,
optional service handle và tests. Không sửa storage/query/executor nếu tuân thủ
generic contract.
