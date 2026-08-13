# Tài liệu thiết kế IFOL GPU

Thư mục này là nguồn thiết kế chính thức của crate `ifol-gpu`. Nó thay thế các tài liệu GPU-specific nằm trong `.agents/design`.

## Thứ tự đọc

1. [Phạm vi và những điều không làm](00-scope-and-non-goals.md)
2. [Kiến trúc](01-architecture.md)
3. [Backend và đa nền tảng](02-backend-and-platform.md)
4. [Mô hình resource](03-resource-model.md)
5. [Mô hình graph và pass](04-graph-and-pass-model.md)
6. [Mô hình command và pipeline](05-command-and-pipeline-model.md)
7. [Memory, synchronization và cache](06-memory-synchronization-and-cache.md)
8. [Validation, error và diagnostics](07-validation-errors-and-diagnostics.md)
9. [Testing và ma trận nền tảng](08-testing-and-platform-matrix.md)
10. [Trạng thái implementation và design debt](09-implementation-status-and-debt.md)
11. [Chiến lược nâng cấp và quyết định rewrite](10-upgrade-strategy.md)
12. [Kế hoạch test bắt buộc](11-test-plan.md)
13. [Ma trận edge case](12-edge-case-matrix.md)
14. [Task plan và thứ tự triển khai](13-task-plan.md)
15. [Definition of Done](14-definition-of-done.md)
16. [Tổng quan Graph Engine](15-graph-engine-overview.md)
17. [Kiến trúc của một Graph](16-graph-architecture.md)
18. [Thuật ngữ và Data Model](17-graph-terms-and-data-model.md)
19. [Flatten và Compile Graph](18-graph-flattening-and-compilation.md)
20. [Sơ đồ kiến trúc và quan hệ](19-graph-architecture-diagram.md)

21. [API validation hiện tại](20-validation-api.md)
22. [Flat render plan API](21-flat-render-plan-api.md)

23. [Execution dependency order](22-execution-dependency-order.md)
24. [Readback theo format](23-readback-format-api.md)
25. [Bundle cache key](24-bundle-cache-key.md)
26. [Texture descriptor validation](25-texture-descriptor-validation.md)
27. [Compute pipeline namespace](26-compute-pipeline-namespace.md)
28. [Compute pass hiện tại](27-compute-pass.md)
29. [Copy pass buffer-to-buffer](28-copy-pass.md)
30. [Graph không có render target](29-non-render-graph-execution.md)
31. [Resource lifecycle API](30-resource-lifecycle-api.md)
32. [Owned texture resource](31-owned-texture-resource.md)
33. [Texture copy pass](32-texture-copy-pass.md)
34. [Thực thi graph theo segment có thứ tự](33-ordered-segment-execution.md)
35. [Buffer descriptor và usage validation](34-buffer-descriptor-and-usage.md)
36. [Ring reset và submission safety](35-ring-reset-and-submission-safety.md)
37. [Validation không panic](36-no-panic-validation.md)
38. [Transient texture pool](37-transient-texture-pool.md)
39. [Registry accessor boundary](38-registry-accessor-boundary.md)
40. [Pipeline mutation qua registry API](39-pipeline-migration.md)
41. [Bind group mutation qua registry API](40-bind-group-migration.md)
42. [Mesh mutation qua registry API](41-mesh-migration.md)
43. [Texture registration qua registry API](42-texture-registration-migration.md)

## Từ vựng trạng thái

- **Đã implement**: đã có và được kiểm chứng trong crate hiện tại;
- **Một phần**: đã có nhưng chưa an toàn, chưa đầy đủ hoặc còn giới hạn;
- **Đã lên kế hoạch**: mục tiêu thiết kế, chưa phải behavior hiện tại;
- **Policy**: optimization hoặc lựa chọn của host, không phải invariant vĩnh viễn.

Khi code và design không khớp, trước hết phải cập nhật tài liệu trạng thái implementation; sau đó giải quyết design decision trước khi thay đổi public contract.
