# Query engine và query plan

## 1. Query là contract dữ liệu

Query không biết phase hay domain. Nó mô tả tập component mà system muốn đọc/ghi.

Khi query chạy trong `SystemContext`, signature của query phát ra access metadata.
ECS resolve `TypeId` thành `ComponentId` của runtime và kiểm tra với
`AccessDescriptor`; query không có trong contract bị từ chối bằng `SystemError`.

~~~mermaid
flowchart LR
    Signature["Query signature"] --> Resolve["Resolve ComponentIds"]
    Resolve --> Driver["Chọn required storage làm driver"]
    Driver --> Filter["Apply required/optional/filter terms"]
    Filter --> Access["Tracked read/write access"]
    Access --> System["SystemContext"]
~~~

Ví dụ contract:

~~~text
Query<&A>
Query<(&A, &B)>
QueryMut<(&mut A, &B)>
Query<(&A, Option<&B>)>
Query<(&A, With<B>, Without<C>)>
WorldRef<T>
Option<WorldRef<T>>
~~~

`Query<Q>` chỉ đọc. `QueryMut<Q>` nhận `&mut World`, kiểm tra aliasing của toàn
bộ signature trước khi tạo iterator và phát ra mutable reference có change tick.
Trong `SystemContext`, cả access contract của system và aliasing của query đều
phải hợp lệ; nếu không, API trả `SystemError`.

Modifier-only queries such as `Option<&T>`, `Without<T>` and `()` iterate the
alive entity set because they do not have a required storage driver. A tuple
chooses the smallest available required storage; an empty required storage
therefore produces an empty result without falling back to an optional term.
The stable tuple convenience implementations cover eight terms; query
composition remains generic through `WorldQuery`.

## 2. WORLD_ENTITY

WORLD_ENTITY được xét như entity bình thường. Query &A trả root nếu root có A.
Nếu muốn loại root, đó là filter explicit của query, không phải hành vi mặc định
của ECS.

## 3. Query plan cache

~~~mermaid
flowchart TD
    Query["Query signature"] --> Key["QueryPlanKey"]
    Key --> Cache{"Cache hit?"}
    Cache -->|"yes"| Plan["Reuse plan"]
    Cache -->|"no"| Build["Build plan from registry/storage"]
    Build --> Store["Store by revision"]
    Store --> Plan
    Plan --> Execute["Iterate/query"]
~~~

QueryPlanKey gồm query signature, component registry revision và structural
version. Value change không làm rebuild plan nếu entity membership không đổi.

## 4. Safety

- Mutable access trùng component bị từ chối.
- Structural mutation trong iterator sống bị hoãn qua Commands.
- Mutable access cập nhật change metadata theo policy của ECS.
- Query rỗng là hợp lệ.
- Required world component thiếu được xử lý bởi RunCondition.
