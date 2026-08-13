# Thực thi graph theo segment có thứ tự

Khi graph chỉ có draw node, compiler có thể dùng một render pass duy nhất để
giảm chi phí encode. Khi graph có cả draw và copy/compute, compiler chuyển sang
execution segmented:

```text
node A: copy/compute  ->  node B: render pass  ->  node C: compute/copy
```

Mỗi draw segment mở một render pass riêng; các lệnh copy và compute được encode
ngay tại vị trí của node trong flat execution order. Vì vậy thứ tự phụ thuộc
được bảo toàn, thay vì gom tất cả copy/compute lên trước render.

Clear color/depth chỉ xảy ra ở render segment đầu tiên; các segment sau dùng
`Load`. Đây là invariant quan trọng để nhiều pass cùng ghi một target không làm
mất kết quả trước đó.

Fast path một render pass vẫn được giữ cho graph thuần render. Bundling hiện
được dùng trong fast path; segmented path ưu tiên tính đúng thứ tự và encode
draw trực tiếp. Tối ưu bundle cho segmented path là task riêng sau khi đã có
benchmark và profiling.
