# World singleton, component data và data flow

## 1. Resource là component trên WORLD_ENTITY

Đăng ký kiểu không tự tạo instance. Feature/Engine có thể cài instance bằng
initializer hoặc provider.

~~~mermaid
flowchart TB
    Type["Component type T"] --> Register["ComponentRegistry"]
    Register --> Install["World API: insert(WORLD_ENTITY, T)"]
    Install --> Store["Normal component storage"]
    Store --> Query["Normal Query / ctx.world_ref<T>"]
~~~

~~~text
init_with(factory)
provided_by_host(instance)
optional
~~~

Không có ResourceRegistry, TypeMap hoặc change tracker song song.

## 2. Cùng một data path

~~~mermaid
sequenceDiagram
    participant H as Host/Feature
    participant E as EcsRuntime
    participant W as World
    participant S as System
    H->>E: register component T
    H->>W: insert WORLD_ENTITY, T
    H->>W: insert entity, T
    S->>W: Query<&T>
    W-->>S: root + normal entities matching T
    S->>W: mutate through tracked access
~~~

WORLD_ENTITY luôn sống và không thể despawn. Cùng type có thể xuất hiện trên root
và entity thường; query trả tất cả entity khớp.

## 3. Required và optional

| Khai báo | Khi vắng mặt | Ý nghĩa |
|---|---|---|
| `RunCondition::WorldHas(A)` | Skip system, ghi reason | Singleton bắt buộc |
| `ctx.world_ref::<A>()?` | System tự xử lý `None` | Singleton tùy chọn |
| Query<&A> | Chạy, iterator có thể rỗng | Query mọi entity có A |
| RequireMatches<Q> | Skip khi query rỗng | Yêu cầu rõ ràng |

## 4. Persistence boundary

~~~mermaid
flowchart LR
    Runtime["ECS runtime component"] --> Metadata["Feature/schema metadata"]
    Metadata --> Persistent["Project serialization"]
    Metadata --> Transient["Transient service handle"]
~~~

Nằm trên WORLD_ENTITY không đồng nghĩa tự động serialize. Schema/Engine bên ngoài
quyết định encode/decode/migration; ECS chỉ giữ runtime data và metadata thay đổi.
