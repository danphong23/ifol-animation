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
registration_revision  → component/system/phase registry thay đổi
graph_revision          → edge/binding/phase ordering thay đổi
structural_version      → entity/component membership thay đổi
component_revision      → component value thay đổi
execution_revision      → mỗi run_once
~~~

Không dùng địa chỉ pointer làm cache key.

## 3. Cache thuộc ECS

- component ID/type lookup cache;
- query plan cache;
- compiled phase order;
- system activation result;
- access/condition validation result;
- execution diagnostics cache tùy chọn.

Render cache, animation cache và media cache thuộc feature, không thuộc ECS core.

## 4. Cache key

~~~text
QueryPlanKey:
    query signature
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
