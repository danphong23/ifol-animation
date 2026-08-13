# IFOL GPU: Chiến lược nâng cấp và quyết định rewrite

## Kết luận

Không nên đập bỏ toàn bộ `ifol-gpu` và viết lại mù từ đầu.

Nên thực hiện **controlled rewrite theo từng lớp**:

1. Giữ implementation hiện tại làm prototype/reference và nguồn visual fixture.
2. Đóng băng việc thêm feature mới vào API cũ.
3. Chốt contract mới bằng tài liệu và test trước.
4. Viết lại từng subsystem có rủi ro cao.
5. Chỉ xóa prototype cũ sau khi implementation mới vượt qua toàn bộ test gate.

## Vì sao chưa cần full rewrite

Các ý tưởng sau có thể giữ lại:

- sử dụng `wgpu` làm portability layer;
- core không phụ thuộc ECS/app;
- handle-based command representation;
- offscreen rendering;
- indexed/procedural draw;
- graph/subgraph concept;
- headless rendering và visual snapshot.

Các phần cần rewrite hoặc refactor mạnh:

- backend builder và feature negotiation;
- resource ownership/registry;
- generational handle;
- graph dependency/compiler;
- render bundle/cache lifecycle;
- frame memory và synchronization;
- error/validation model;
- surface/readback abstraction.

## Nguyên tắc migration

- Không thay đổi nhiều subsystem trong một task.
- Mỗi thay đổi public API phải đi kèm test hoặc compile-fail expectation.
- Không dùng benchmark để chứng minh correctness.
- Không dùng visual output làm bằng chứng duy nhất cho memory safety.
- Không xóa API cũ trước khi có migration note hoặc replacement API.
- Không gọi feature là hoàn thành nếu chưa có test tương ứng.

## Các giai đoạn

### Phase 0 — Baseline và đóng băng

Mục tiêu: biết chính xác trạng thái hiện tại.

- ghi nhận commit/reference prototype;
- sửa hoặc cô lập examples đang lệch API;
- tạo test command chuẩn;
- lưu các visual fixture hiện có;
- phân loại mọi tính năng theo `implemented`, `partial`, `planned`.

Gate: unit test, library check và test harness baseline chạy được; mọi lỗi còn lại được ghi thành issue/task cụ thể.

### Phase 1 — Contract và lỗi

Mục tiêu: API không còn silent failure.

- thiết kế typed error;
- validate handle, slot, range, attachment;
- loại bỏ silent skip resource thiếu;
- bổ sung builder setter cho backend/features/limits;
- sửa backend selection thực sự được áp dụng.

Gate: toàn bộ validation và error tests pass; không có panic với invalid public input.

### Phase 2 — Resource và handle

Mục tiêu: resource lifetime có thể tin cậy.

- generational handles;
- đóng raw `HashMap` public;
- resource store có version;
- descriptor đầy đủ;
- explicit create/replace/destroy;
- deferred destruction theo submission completion.

Gate: stale-handle, wrong-type, replace, destroy và resource-lifetime tests pass.

### Phase 3 — Frame memory và synchronization

Mục tiêu: không ghi đè dữ liệu GPU đang dùng.

- frame context;
- submission tracking;
- ring/upload allocator an toàn;
- transient resource pool;
- async readback contract;
- overflow/alignment handling.

Gate: memory edge-case tests pass; test nhiều frame in-flight không corruption.

### Phase 4 — Graph/pass model

Mục tiêu: graph trở thành dependency-aware nhưng vẫn đơn giản.

- pass abstraction;
- resource read/write usage;
- dependency validation và cycle detection;
- render target/attachment descriptor;
- render, compute và copy pass boundary;
- compiler execution plan.

Gate: topology, hazard, ordering và pass execution tests pass.

### Phase 5 — Pipeline, binding và cache

Mục tiêu: compiled artifact đúng theo context.

- bind-group limit động;
- pipeline compatibility validation;
- bundle cache ngoài logical node;
- context/resource version key;
- dynamic offset không bị bake sai;
- MSAA/resolve policy.

Gate: cache hit/miss/invalidation và dynamic-data tests pass; visual tests vẫn pass.

### Phase 6 — Platform và capability

Mục tiêu: portable behavior có bằng chứng.

- surface format lấy từ configuration;
- headless/native/web separation;
- capability tier;
- backend/platform matrix;
- surface loss/resize/present handling.

Gate: mỗi platform khả dụng chạy baseline suite; platform không hỗ trợ feature phải trả về capability error rõ ràng.

### Phase 7 — Performance và cleanup

Mục tiêu: tối ưu sau khi correctness ổn định.

- tách CPU build/compile/encode/submit/GPU wait benchmark;
- profile allocation và cache;
- tối ưu command storage;
- xem xét state sorting policy;
- xóa prototype code/dead API;
- cập nhật README/FEATURES/docs.

Gate: không regression correctness; benchmark có phương pháp đo và baseline được ghi lại.

## Tiêu chí quyết định full rewrite

Chỉ full rewrite nếu xảy ra một trong các điều kiện:

- public model không thể biểu diễn dependency/resource usage cần thiết;
- ownership hiện tại tạo ra unsoundness không thể cô lập;
- API cũ cản trở platform hoặc compute/copy pass;
- migration từng lớp tạo nhiều compatibility hack hơn code mới;
- test chứng minh invariant nền tảng không thể sửa mà không phá toàn bộ model.

Hiện tại chưa có bằng chứng đủ mạnh cho các điều kiện trên.
