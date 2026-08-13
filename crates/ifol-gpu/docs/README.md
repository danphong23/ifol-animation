# Tài liệu thiết kế IFOL GPU

Thư mục này là nguồn thiết kế chính thức của crate `ifol-gpu`. Nó thay thế các tài liệu GPU-specific nằm trong `.agents/design`.

## Muc luc theo nhom

### 00-foundation
1. [quy ước tổ chức tài liệu](00-foundation/02-docs-organization.md)
2. [phân loại phạm vi core và engine](00-foundation/15-core-boundaries-and-task-map.md)
3. [scope and non goals](00-foundation/00-scope-and-non-goals.md)
4. [architecture](00-foundation/01-architecture.md)
5. [validation errors and diagnostics](00-foundation/07-validation-errors-and-diagnostics.md)
6. [testing and platform matrix](00-foundation/08-testing-and-platform-matrix.md)
7. [upgrade strategy](00-foundation/10-upgrade-strategy.md)
8. [test plan](00-foundation/11-test-plan.md)
9. [edge case matrix](00-foundation/12-edge-case-matrix.md)
10. [task plan](00-foundation/13-task-plan.md)
11. [definition of done](00-foundation/14-definition-of-done.md)

### 10-graph
1. [graph and pass model](10-graph/04-graph-and-pass-model.md)
2. [graph engine overview](10-graph/15-graph-engine-overview.md)
3. [graph architecture](10-graph/16-graph-architecture.md)
4. [graph terms and data model](10-graph/17-graph-terms-and-data-model.md)
5. [graph flattening and compilation](10-graph/18-graph-flattening-and-compilation.md)
6. [graph architecture diagram](10-graph/19-graph-architecture-diagram.md)
7. [flat render plan api](10-graph/21-flat-render-plan-api.md)
8. [execution dependency order](10-graph/22-execution-dependency-order.md)
9. [ordered segment execution](10-graph/33-ordered-segment-execution.md)
10. [graph resource usage](10-graph/45-graph-resource-usage.md)
11. [automatic hazard edges](10-graph/46-automatic-hazard-edges.md)
12. [inferred pass usage](10-graph/47-inferred-pass-usage.md)
13. [nested graph dependencies](10-graph/48-nested-graph-dependencies.md)
14. [texture subresource hazards](10-graph/58-texture-subresource-hazards.md)
15. [texture copy aspect](10-graph/71-texture-copy-aspect.md)
16. [no compile order fallback](10-graph/83-no-compile-order-fallback.md)
17. [flat plan execution](10-graph/84-flat-plan-execution.md)

### 20-resources
1. [resource model](20-resources/03-resource-model.md)
2. [bundle cache key](20-resources/24-bundle-cache-key.md)
3. [texture descriptor validation](20-resources/25-texture-descriptor-validation.md)
4. [resource lifecycle api](20-resources/30-resource-lifecycle-api.md)
5. [owned texture resource](20-resources/31-owned-texture-resource.md)
6. [texture copy pass](20-resources/32-texture-copy-pass.md)
7. [buffer descriptor and usage](20-resources/34-buffer-descriptor-and-usage.md)
8. [registry accessor boundary](20-resources/38-registry-accessor-boundary.md)
9. [dynamic bind group limit](20-resources/70-dynamic-bind-group-limit.md)
10. [registry deferred ownership](20-resources/74-registry-deferred-ownership.md)
11. [bundle cache sample context](20-resources/76-bundle-cache-sample-context.md)
12. [dynamic offset descriptor validation](20-resources/77-dynamic-offset-descriptor-validation.md)
13. [pipeline layout signature validation](20-resources/78-pipeline-layout-signature-validation.md)
14. [context aware bundle cache](20-resources/79-context-aware-bundle-cache.md)
15. [texture descriptor structural validation](20-resources/85-texture-descriptor-structural-validation.md)

### 30-execution
1. [command and pipeline model](30-execution/05-command-and-pipeline-model.md)
2. [validation api](30-execution/20-validation-api.md)
3. [readback format api](30-execution/23-readback-format-api.md)
4. [compute pipeline namespace](30-execution/26-compute-pipeline-namespace.md)
5. [compute pass](30-execution/27-compute-pass.md)
6. [copy pass](30-execution/28-copy-pass.md)
7. [non render graph execution](30-execution/29-non-render-graph-execution.md)
8. [no panic validation](30-execution/36-no-panic-validation.md)
9. [indirect execution](30-execution/59-indirect-execution.md)
10. [async readback](30-execution/64-async-readback.md)
11. [execution report and profiling hook](30-execution/65-execution-report-and-profiling-hook.md)
12. [timestamp query pool](30-execution/67-timestamp-query-pool.md)
13. [executor timestamp boundary](30-execution/68-executor-timestamp-boundary.md)

### 40-memory
1. [memory synchronization and cache](40-memory/06-memory-synchronization-and-cache.md)
2. [ring reset and submission safety](40-memory/35-ring-reset-and-submission-safety.md)
3. [transient texture pool](40-memory/37-transient-texture-pool.md)
4. [transient buffer pool](40-memory/60-transient-buffer-pool.md)
5. [deferred destruction](40-memory/61-deferred-destruction.md)
6. [frame context](40-memory/62-frame-context.md)
7. [tracked profiling lifecycle](40-memory/73-tracked-profiling-lifecycle.md)
8. [frame owned deferred lifecycle](40-memory/75-frame-owned-deferred-lifecycle.md)

### 50-platform
1. [backend and platform](50-platform/02-backend-and-platform.md)
2. [no panic surface locks](50-platform/44-no-panic-surface-locks.md)
3. [capability requirements](50-platform/49-capability-requirements.md)
4. [builder platform policy](50-platform/50-builder-platform-policy.md)
5. [msaa resolve boundary](50-platform/51-msaa-resolve-boundary.md)
6. [surface initialization contract](50-platform/52-surface-initialization-contract.md)
7. [surface resize contract](50-platform/53-surface-resize-contract.md)
8. [checked surface execution](50-platform/54-checked-surface-execution.md)
9. [execution api migration](50-platform/55-execution-api-migration.md)
10. [surface reconfigure](50-platform/63-surface-reconfigure.md)
11. [timestamp capability and platform fallback](50-platform/66-timestamp-capability-and-platform-fallback.md)
12. [platform compile evidence](50-platform/69-platform-compile-evidence.md)
13. [fallback adapter policy](50-platform/72-fallback-adapter-policy.md)

### 60-guides
1. [guide getting started](60-guides/87-guide-getting-started.md)
2. [guide pipeline and shader](60-guides/88-guide-pipeline-and-shader.md)
3. [guide building a graph](60-guides/89-guide-building-a-graph.md)
4. [guide public api and extensions](60-guides/90-guide-public-api-and-extensions.md)

### 70-status
1. [implementation status and debt](70-status/09-implementation-status-and-debt.md)
2. [current audit](70-status/80-current-audit.md)
3. [typed readback errors](70-status/81-typed-readback-errors.md)
4. [typed texture save errors](70-status/82-typed-texture-save-errors.md)
5. [core cleanup and roadmap](70-status/86-core-cleanup-and-roadmap.md)

### 80-migration
1. [pipeline migration](80-migration/39-pipeline-migration.md)
2. [bind group migration](80-migration/40-bind-group-migration.md)
3. [mesh migration](80-migration/41-mesh-migration.md)
4. [texture registration migration](80-migration/42-texture-registration-migration.md)
5. [private resource store](80-migration/43-private-resource-store.md)

## Tu vung trang thai

- **Da implement**: da co va da duoc kiem chung;
- **Mot phan**: da co nhung chua day du hoac con gioi han;
- **Da len ke hoach**: muc tieu thiet ke, chua phai behavior hien tai;
- **Policy**: lua chon cua host, khong phai invariant vinh vien.
> Quy ước cấu trúc và quy trình cập nhật docs: [02-docs-organization](00-foundation/02-docs-organization.md).
