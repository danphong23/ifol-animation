# IFOL GPU: execution report và profiling hook

## Mục tiêu

Core cần cho host biết một graph đã flatten thành gì và đã submit ra sao,
nhưng không được tự biến thành hệ thống profiler phụ thuộc nền tảng. Vì vậy
core cung cấp số liệu cấu trúc ổn định; host có thể nối timestamp query,
tracing hoặc telemetry riêng sau này.

## API

`RenderGraphExecutor::execute_checked_with_report` và
`execute_with_surface_checked_with_report` trả `ExecutionReport`. Các API
`execute` hiện tại vẫn chỉ trả `SubmissionIndex` để giữ tương thích.

Report gồm:

- `submission`: submission index vừa được queue nhận;
- `flattened_nodes`: số node trong flat plan, bao gồm node con của nested graph;
- `draw_commands`, `compute_commands`, `copy_commands`;
- `indirect_commands`: tổng draw/dispatch indirect;
- `declared_usages`: số usage declaration explicit được gắn trên các node.

Các số liệu được thu sau validation và trước submit. Chúng mô tả command model,
không phải thời gian GPU thực tế. Không được dùng report để suy ra chắc chắn số
render pass hoặc chi phí backend.

## Mở rộng có kiểm soát

Profiling timestamp là task riêng vì cần capability query, query-set lifetime,
resolve buffer và policy đọc kết quả theo submission. API report hiện tại tạo
điểm móc để thêm các số liệu đó mà không thay đổi graph model hay buộc host dùng
một profiler cụ thể.

## Test gate

Test unit dựng graph có draw/compute/copy node và explicit usage, sau đó kiểm tra
flat node count, từng loại command, indirect count và usage count.

