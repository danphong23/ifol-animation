# Cache, revision và invalidation

## 1. Nguyên tắc từ ifol-gpu

Cache là optimization, không phải source of truth. Logical World, registration và
phase graph luôn có thể dùng để rebuild.

~~~mermaid
flowchart LR
    Source["World + logical registrations"] --> Compile["Compile plan"]
    Compile --> Cache["Compiled/query/activation cache"]
    Cache --> Hit["Revision match: reuse"]
    Cache --> Miss["Revision mismatch: rebuild"]
    Source --> Fallback["Correctness fallback"]
    Fallback --> Execute["Same semantic execution"]
    Hit --> Execute
    Miss --> Execute
~~~

## 2. Các revision chính

~~~text
component_registry_revision → component type registration thay đổi
system_registry_revision    → system registration thay đổi
graph_revision              → phase/edge/binding ordering thay đổi
structural_version      → entity/component membership thay đổi
component_change_tick   → từng component entry bị mutable write
execution_revision      → mỗi run_once
~~~

Không dùng địa chỉ pointer làm cache key.

## 3. Cache thuộc ECS

- component ID/type lookup cache;
- query plan cache;
- compiled phase order;
- access/condition validation result (compile-time check, chưa cache riêng).
- execution diagnostics cache tùy chọn.

Render cache, animation cache và media cache thuộc feature, không thuộc ECS core.

## 4. Cache key

~~~text
QueryPlanKey:
    concrete query type/signature
    component type IDs
    component registry revision
    structural version

SchedulePlanKey:
    registration revision
    graph revision
    access/condition policy

ActivationKey:
    system id
    structural version
    relevant world-component presence
~~~

Nếu component value thay đổi nhưng membership không đổi, query plan không cần
rebuild. Feature system tự dùng component revision để invalidate artifact domain.

## 5. Recompile và lifetime

Recompile tạo plan mới nhưng giữ World data. Cache cũ phải drop hoặc stale; không
giữ reference vào storage cũ.
