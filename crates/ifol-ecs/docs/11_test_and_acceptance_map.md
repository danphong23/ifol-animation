# Test và acceptance map

## Test pipeline

~~~mermaid
flowchart LR
    Unit["Unit: identity/storage/query"] --> Integration["Integration: runtime registration"]
    Integration --> Feature["Test feature: phase/system graph"]
    Feature --> Stress["Stress/property: churn + revisions"]
    Stress --> Profile["Benchmark: query/compile/cache/run"]
~~~

## Acceptance slices

| Slice | Chứng minh |
|---|---|
| Entity | generation, alive/free, WORLD_ENTITY, stale ID |
| Component | registry, storage, replace/remove, drop/lifetime |
| World singleton | root query, required/optional provider |
| Query | tuple, optional, mutable, filter, empty result |
| Registry | duplicate/missing IDs, transactional commit |
| Phase graph | edges, cycle, deterministic topological order |
| System | context access, errors, no phase knowledge |
| Execute | safe point, report, re-run/recompile |
| Cache | hit/miss/invalidation, rebuild correctness |
| Extension | feature registration không sửa core |
| Lifecycle | clear, reconfigure, shutdown |

## Definition of Done

Core chỉ hoàn chỉnh khi mọi slice có test, public API không cần raw storage access,
cùng input/revision tạo execution order deterministic, cache miss cho cùng semantic
result, invalid input có typed diagnostic và ifol-ecs không chứa domain
component/phase.

Correctness test không tự ghi file vào source tree. Benchmark/profiling là nhóm riêng.
