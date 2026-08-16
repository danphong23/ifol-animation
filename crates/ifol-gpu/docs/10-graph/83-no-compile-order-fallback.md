# IFOL GPU: không fallback thứ tự khi compile graph lỗi

`RenderGraphExecutor` luôn validate graph trước khi encode. Trước đây
`compile_graph` vẫn dùng declaration order nếu topo ordering lỗi, khiến invariant
này có thể bị phá khi sau này có thêm call path.

Hiện `compile_graph` trả `Result<(), RenderGraphValidationError>` và map trực tiếp
`GraphFlattenError` thành lỗi typed. Cycle, node thiếu hoặc dependency ngoài
graph không còn bị thay bằng thứ tự declaration.

Các encoder helper vẫn có nhánh `continue` cho implementation detail, nhưng
chúng chỉ được gọi sau validation thành công. Đây là boundary cố ý: public
invalid input trả lỗi; private encode không lặp lại toàn bộ validation.

## Test gate

`public_execute_checked_rejects_invalid_graph_before_submit` và toàn bộ graph cycle/
missing-node tests tiếp tục là evidence cho boundary này.
