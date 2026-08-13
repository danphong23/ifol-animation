# IFOL GPU: Mô hình graph và pass

## Mục tiêu

Graph là mô tả rõ ràng về GPU work và resource dependency. Nó phải hỗ trợ 2D, 2.5D, 3D, compositing, compute effect và offline rendering mà không chứa semantic của domain.

## Loại pass

Mô hình mục tiêu có các pass riêng:

- `RenderPass`: raster draw command và attachment;
- `ComputePass`: dispatch command và storage resource;
- `CopyPass`: buffer/texture copy và upload;
- `Resolve/GenerateMips`: thao tác transfer cần thiết;
- `PresentPass`: bước presentation cho host.

Implementation ban đầu có thể chỉ hỗ trợ render pass, nhưng graph representation không được khiến các loại pass còn lại trở nên bất khả thi.

## Dependency

Mỗi pass khai báo resource mà nó đọc và ghi. Compiler validate hazard và suy ra execution order. Graph đơn giản có thể được biểu diễn bằng ordered list, nhưng semantic thực sự là read/write dependency, không chỉ là vị trí của child node.

Cycle, resource thiếu, usage không tương thích và attachment combination sai đều là compile error.

## Thứ tự render

Ordering phải rõ ràng. Mặc định core phải giữ nguyên thứ tự. Policy tùy chọn có thể reorder command được khai báo là order-independent, ví dụ opaque state sorting. Transparent và painter-order work không được reorder bằng optimization chung.

## Subgraph

Subgraph là tiện ích để compose graph description. Nó không bắt buộc phải tương ứng với cây `RenderNode` hay một số lượng pass cụ thể. Subgraph tạo ra image phải khai báo output resource và usage. Compiler có thể flatten, giữ nguyên hoặc split tùy dependency và target compatibility.

## Target và attachment

Target mô tả attachment thật hoặc presentation context. Format, sample count, load/store operation, resolve target, depth/stencil operation và viewport/scissor phải rõ ràng hoặc được suy ra từ resource descriptor đã validate.

## Trạng thái implementation hiện tại

`RenderGraph` hiện là ordered collection của node ID có nesting tùy chọn. Nó hữu ích cho prototype, nhưng chưa phải dependency-aware render graph. Vì vậy quy tắc “một graph bằng một render pass” chỉ là policy của implementation hiện tại, không phải guarantee vĩnh viễn.
