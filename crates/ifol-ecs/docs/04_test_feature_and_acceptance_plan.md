# Test feature và acceptance plan

## 1. Mục đích

Core không chứa component/system nghiệp vụ, nhưng cần một bộ feature giả đủ rộng
để kiểm chứng contract. Bộ này đăng ký qua EcsRuntime API thật, đặt trong
tests/support hoặc crate dev-only, không export như production và không kéo
GPU/schema/engine vào test.

~~~mermaid
flowchart LR
    Fixture["Test components + systems"] --> API["EcsRuntime API"]
    API --> Compile["register + compile"]
    Compile --> Run["run_once"]
    Run --> Assert["World + RunReport assertions"]
~~~

## 2. Test data và systems

Component entity mẫu:

- `Position`, `Velocity`, `Health`, `Name`, `OptionalTag`;
- kiểu zero-sized và kiểu có drop counter để kiểm tra lifecycle.

Singleton component mẫu trên `WORLD_ENTITY`:

- `TestConfig` — dependency bắt buộc của một số system;
- `RunCounter` — state được system cập nhật;
- `MockServiceHandle` — dữ liệu transient, không đòi serialization.

System mẫu:

- movement: `(&mut Position, &Velocity)`;
- optional join: `(&Position, Option<&OptionalTag>)`;
- multi-read/write và query 0 match;
- required singleton với `WorldHas<TestConfig>`;
- optional singleton: vẫn chạy khi config vắng;
- `WorldHas` và tổ hợp `All`/`Any` để chứng minh skip theo yêu cầu rõ;
- deferred spawn/despawn/add/remove;
- system cố ý trả error để kiểm tra execution policy.

## 3. Acceptance matrix

### World và storage

- stale generational ID không đọc/xóa entity tái sử dụng slot;
- `WORLD_ENTITY` không despawn/recycle nhưng insert/remove component được;
- swap-remove giữ sparse/dense mapping chính xác;
- despawn drop mỗi component đúng một lần;
- type chưa đăng ký trả diagnostic rõ.
- forged next-generation ID trong slot free bị từ chối.

### Query

- single, tuple, mutable và optional query trả đúng tập;
- `WORLD_ENTITY` xuất hiện trong query thường khi khớp;
- optional term không lọc entity;
- query 0 match không lỗi;
- `WORLD_ENTITY` xuất hiện trong query thường khi khớp;
- mutable query và alias rejection;
- alias mutable bất hợp lệ bị từ chối;
- structural mutation qua deferred commands chỉ có hiệu lực ở safe point.

### Change tracking

- mỗi component entry có last-change tick đúng;
- sửa singleton dùng cùng tracking với entity thường;
- structural version chỉ đổi bởi thay đổi cấu trúc;
- tick tăng theo execution pass và mutable access cập nhật tick.

### Schedule

- duplicate ID, missing dependency và phase cycle làm build lỗi;
- topological tie-break cho kết quả ổn định;
- access descriptor tự mâu thuẫn bị từ chối;
- thiếu component type ở condition làm build lỗi;
- thiếu singleton bắt buộc làm skip có reason;
- insert singleton làm system active, remove làm skip lại;
- optional singleton vắng vẫn chạy;
- query rỗng mặc định vẫn chạy; `WorldHas` thiếu thì skip;
- system error và deferred command outcome xuất hiện trong `RunReport`.

### Cache và lifecycle

- cùng revision dùng lại plan/cache hợp lệ;
- structural/data/graph revision invalidates đúng cache;
- recompile không làm mất World data;
- cache miss cho cùng logical input cho cùng semantic result;
- correctness test không ghi report vào source tree; benchmark nằm trong `benches/`.

## 4. Bậc kiểm thử

1. Unit test cho identity/storage/metadata.
2. Integration test cho query và scheduler bằng test feature.
3. Property/stress test cho chuỗi spawn/despawn/add/remove và DAG ngẫu nhiên.
4. Benchmark riêng, không trộn với correctness test.
5. Property/benchmark mở rộng có thể bổ sung sau; không được biến thành dependency
   của core runtime hoặc ghi artifact vào source tree.

Definition of Done của core là acceptance matrix xanh và public contract được
ghi lại; không phải số lượng domain feature đã xây.
