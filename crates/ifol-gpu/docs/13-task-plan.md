# IFOL GPU: Task plan và thứ tự triển khai

## Quy tắc task

Mỗi task phải có:

- mục tiêu rõ;
- file/module dự kiến ảnh hưởng;
- test mới hoặc test được cập nhật;
- điều kiện pass;
- không mở rộng scope ngoài task.

## Tầng 0: Baseline

### T0.1 — Chốt baseline build

- Sửa/cô lập examples và benchmarks đang dùng API cũ.
- Xác định lệnh test chuẩn.
- Ghi nhận lỗi môi trường GPU khác với lỗi code.

Test gate: toàn bộ library test, integration test và example compile có status rõ ràng.

### T0.2 — Đóng băng API prototype

- Ghi rõ API nào provisional.
- Không thêm feature mới vào raw registry/node bundle API.

Test gate: compile và regression baseline pass.

## Tầng 1: Correctness contract

### T1.1 — Structured error

Test: missing adapter, missing resource, invalid slot, invalid range, invalid target.

### T1.2 — Backend/features/limits builder

Test: backend config được consume; required feature/limit success/failure.

### T1.3 — Surface context

Test: format lấy từ surface config; resize, zero size, lost/outdated.

Gate tầng 1: không hard-code backend/surface format; invalid public input không panic.

## Tầng 2: Resource model

### T2.1 — Generational handle

Test: stale, wrong type, destroy/recreate, generation overflow policy.

### T2.2 — Resource store

Test: create/get/replace/remove/version và lookup sau deferred destroy.

### T2.3 — Full descriptors

Test: texture/buffer descriptor compatibility, usage, size, sample, mip.

Gate tầng 2: mọi resource reference đều validate được.

## Tầng 3: Memory/synchronization

### T3.1 — Submission tracker

Test: complete/not complete, multiple in-flight submissions, deferred release.

### T3.2 — Frame/upload allocator

Test: alignment, wrap, capacity, overflow, reuse sau completion.

### T3.3 — Transient pool

Test: exact descriptor reuse, incompatible descriptor miss, in-flight protection.

Gate tầng 3: stress nhiều frame không overwrite hoặc reuse sớm.

## Tầng 4: Graph/compiler

### T4.1 — Pass/resource usage model

Test: read/write declaration, missing resource, cycle, hazard.

### T4.2 — Execution plan

Test: linear, fan-in, fan-out, nested graph, independent pass ordering.

### T4.3 — Render/compute/copy boundary

Test: render baseline trước; compute/copy chỉ mở khi resource model đã pass.

Gate tầng 4: compiler không phụ thuộc `1 graph = 1 pass`.

## Tầng 5: Pipeline/cache

### T5.1 — Dynamic bind-group state

Test: device bind-group limit, slot validation, dynamic offset.

### T5.2 — Context-aware bundle cache

Test: cache key, hit/miss, format/depth/sample/pipeline invalidation.

### T5.3 — Static/dynamic command split

Test: dynamic frame data không bị bake vào static bundle.

Gate tầng 5: visual regression và cache tests pass.

## Tầng 6: Portability

### T6.1 — Headless matrix

Test: baseline render trên backend khả dụng.

### T6.2 — Surface matrix

Test: resize, format, present và surface loss.

### T6.3 — Capability tiers

Test: feature unsupported trả fallback/error rõ ràng.

Gate tầng 6: không tuyên bố platform parity nếu chưa có test evidence.

## Tầng 7: Performance/cleanup

### T7.1 — Benchmark methodology

Tách CPU build, compile, encode, submit và GPU completion.

### T7.2 — Remove prototype debt

- bỏ hard-code;
- bỏ public raw map;
- bỏ silent skip;
- đổi tên cache sai semantics;
- xóa code chết và docs cũ mâu thuẫn.

Gate tầng 7: full test suite pass, docs/status cập nhật, không regression.
