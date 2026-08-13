# Automatic hazard edges

`RenderGraph::ordered_node_ids` giờ kết hợp hai loại dependency:

1. explicit dependency do host khai báo;
2. implicit hazard edge do `ResourceUsage` tạo ra.

Nếu hai node cùng dùng một `GraphResource` và ít nhất một access là `Write` hoặc
`ReadWrite`, compiler thêm edge theo declaration order. Hai node chỉ `Read`
không tạo edge vì có thể chạy độc lập.

Explicit dependency ngược với hazard edge tạo cycle và bị từ chối trước encode.
Quy tắc này là bước đầu của hazard model; resource state transition/backend
barrier và subresource range vẫn cần bổ sung sau.
