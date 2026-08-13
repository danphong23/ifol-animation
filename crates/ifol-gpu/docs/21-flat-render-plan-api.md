# IFOL GPU: Flat render plan API

`RenderGraph::flatten(&RenderNodePool)` chuyển logical graph lồng nhau thành
`FlatRenderPlan`. Mỗi phần tử là `FlatRenderNode`, gồm `node_id` và `path` từ root
đến node hiện tại.

Thứ tự hiện tại là bottom-up:

```text
Root
├─ ChildGraph
│  └─ ChildPass
└─ Composite

FlatRenderPlan = ChildPass → ChildGraph → Composite
```

API này chỉ tạo logical plan và không submit GPU. Nó đã phát hiện node thiếu và
cycle theo active path. Graph có thể khai báo dependency explicit; plan dùng
topological order ổn định với declaration order làm tie-breaker và báo cycle hoặc
dependency trỏ ra ngoài graph. Resource usage/lifetime/scheduling vẫn là các bước
compiler tiếp theo.
