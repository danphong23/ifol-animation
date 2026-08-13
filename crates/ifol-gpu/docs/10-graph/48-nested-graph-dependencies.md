# Dependency của nested graph trong flat plan

`RenderGraph::flatten` thu thập explicit dependency của root và mọi subgraph,
sau đó áp dụng chung trên `FlatRenderPlan`. Dependency của child graph được
validate trong đúng scope của child graph; node không thuộc graph đó bị báo
`DependencyNodeOutsideGraph`.

Điều này bảo đảm flatten không làm mất ordering do nested graph khai báo. Cùng
pipeline còn kết hợp hazard edges và declaration order, nên thứ tự cuối cùng là
một plan phẳng duy nhất có thể dùng cho diagnostics/scheduling.
