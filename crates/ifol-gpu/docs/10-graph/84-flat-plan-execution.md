# IFOL GPU: thực thi theo flat execution plan

`RenderGraph::flatten` không còn chỉ được dùng cho diagnostics và execution
report. Boundary compile chính dùng chính plan này khi graph có nested node.

## Quy tắc

- Graph trực tiếp, không nested và không bị reorder, được dùng fast path cũ để
  giữ một render pass/bundle cache khi có thể.
- Graph có nested node hoặc flat order khác declaration order được encode theo
  đúng thứ tự `FlatRenderPlan`.
- Mỗi flat node được resolve về graph sở hữu nó qua `FlatRenderNode::path`.
  Command của node con chạy trên target của graph con; command composite của
  `SubGraph` chạy trên target của graph cha.
- Copy và compute được encode tại vị trí của node trong flat plan, không bị gom
  lên trước render pass của graph cha.
- Với cùng một target, clear chỉ xảy ra ở draw đầu tiên; MSAA resolve chỉ xảy
  ra ở draw cuối của target đó.

## Vì sao cần boundary này?

Nếu root graph khai báo một upload trước `SubGraph`, nhưng executor luôn recurse
subgraph trước root, nested compute có thể đọc dữ liệu cũ. Flat execution giữ
đúng quan hệ declaration/hazard mà compiler đã tính.

Test runtime hồi quy:

```text
root copy 7 -> nested compute (+1) -> root readback = 8
```

## Giới hạn có chủ ý

Fast path chỉ được chọn khi flat plan trùng hoàn toàn với danh sách node trực
tiếp của graph. Vì vậy nó không thể bỏ qua dependency, hazard hoặc nested
ordering. Bundle tối ưu cho segmented flat path là một task riêng; correctness
được ưu tiên trước cache optimization ở boundary này.

