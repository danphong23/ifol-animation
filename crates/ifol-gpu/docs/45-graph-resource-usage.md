# IFOL GPU: khai báo resource usage trong graph

Mỗi node có thể khai báo `GraphResource`, `ResourceAccess` và subresource.
Texture hỗ trợ mip/layer/aspect; buffer hỗ trợ byte range. Compiler dùng metadata
để tạo hazard edge giữa write/read hoặc các access overlap.

Copy, indirect draw/dispatch và attachment hiện có inferred usage built-in. Host
vẫn phải khai báo usage cho storage/resource binding mà core không thể suy luận
từ shader opaque.

Graph flatten dùng usage map để topo-sort ổn định. Hai texture aspect disjoint
không tạo hazard; `All` overlap depth/stencil; range disjoint có thể chạy độc
lập. Đây là dependency logic ở core, không phải backend barrier API riêng.

Validation vẫn từ chối resource thiếu, range/aspect sai và usage descriptor thiếu
trước submit. Chi tiết hazard tự động xem [automatic hazard edges](46-automatic-hazard-edges.md).
