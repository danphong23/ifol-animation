# Entity, World và component storage

## 1. Entity là gì?

Entity là identity, không phải object chứa sẵn field domain.

~~~mermaid
flowchart LR
    Id["EntityId<br/>index + generation"] --> Slot["Entity slot state<br/>alive / free"]
    Slot --> Components["Component storages<br/>lookup theo EntityId"]
~~~

EntityId gồm index và generation. Generation bảo vệ handle cũ khi slot được tái sử
dụng. Entity manager phải giữ trạng thái alive riêng; chỉ so sánh generation là
chưa đủ.

## 2. WORLD_ENTITY

~~~mermaid
flowchart TB
    World["World"] --> Root["WORLD_ENTITY<br/>reserved, luôn alive"]
    World --> E1["Entity A"]
    World --> E2["Entity B"]
    Root --> R["Config · ServiceHandle · bất kỳ component T"]
    E1 --> A["Position · Shape"]
    E2 --> B["Position · Image"]
~~~

WORLD_ENTITY được tạo khi World khởi tạo, không despawn/free/recycle, vẫn là entity
bình thường đối với insert/get/remove/query và dùng cùng change tracking.

Resource/world singleton chỉ là quy ước: T nằm trên WORLD_ENTITY.

## 3. World sở hữu dữ liệu gì?

~~~text
World
├── EntityManager
├── ComponentRegistry
├── Type-erased component stores
├── WORLD_ENTITY component set
├── structural_version
├── change/revision state
└── command application state
~~~

World không biết T là Position, Time hay GPU handle. Nó chỉ biết runtime ComponentId
và storage tương ứng.

## 4. Storage contract

Mỗi component type có một storage riêng về implementation. SparseSet là lựa chọn
hiện tại, nhưng không phải domain contract:

~~~text
ComponentId<T>
    ├── sparse index: EntityId.index → dense slot
    ├── dense entities
    ├── dense component data
    └── per-entry change metadata
~~~

Invariant:

- stale EntityId không đọc entity mới;
- dead slot không được insert;
- swap-remove cập nhật sparse pointer;
- despawn remove component từ mọi store;
- public mutation luôn đi qua change-aware boundary.

## 5. Resource và persistence

Không có ResourceRegistry riêng:

~~~text
world.insert(WORLD_ENTITY, Config)
world.get::<Config>(WORLD_ENTITY)
world.query::<&Config>()  // có thể trả WORLD_ENTITY
~~~

Registration metadata có thể nói component nào là world singleton, nhưng storage
vẫn là component storage. Việc tự tạo instance, persistence/transient và migration
thuộc Engine/Feature/Schema; ECS chỉ lưu runtime data.

## 6. Structural và data revision

~~~mermaid
flowchart LR
    Structural["spawn/despawn/add/remove"] --> SV["structural_version"]
    Data["mutable component write"] --> CV["component change revision"]
    SV --> QueryPlan["invalidate query/activation plan"]
    CV --> Consumers["change-aware systems/cache"]
~~~

Sửa giá trị không nhất thiết làm thay đổi entity nào khớp query; thêm/xóa component
thì có. Không dùng một counter duy nhất cho hai semantics này.
