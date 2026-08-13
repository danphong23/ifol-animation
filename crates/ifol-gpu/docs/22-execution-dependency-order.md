# IFOL GPU: execution dùng dependency order

`RenderGraph::ordered_node_ids` cung cấp thứ tự node trực tiếp sau khi áp dụng
dependency explicit. `RenderGraphExecutor` dùng thứ tự này khi tạo bundle và render
pass, nên dependency không chỉ tồn tại trong logical `FlatRenderPlan`.

Declaration order vẫn là tie-breaker deterministic. Dependency cycle hoặc dependency
trỏ ra ngoài graph bị `execute_checked` từ chối thông qua validation trước submit.
