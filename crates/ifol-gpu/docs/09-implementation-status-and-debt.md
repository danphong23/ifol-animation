# IFOL GPU: Trạng thái implementation và design debt

Tài liệu này ngăn prototype behavior bị hiểu nhầm là contract đã hoàn thiện.

## Prototype đã có

- khởi tạo `wgpu` device/queue;
- capability snapshot cơ bản;
- screen/offscreen render target;
- indexed và procedural draw command;
- ordered graph có nesting;
- depth attachment và clear color;
- bind group cơ bản và dynamic offset;
- uniform ring allocation cơ bản, không còn implicit wrap ghi đè allocation cũ;
- texture readback và image export utility;
- render example và benchmark scaffold.

## Đã có một phần

- multi-viewport reuse: có logical reuse nhưng compiled artifact chưa an toàn theo context;
- render bundle cache: có nhưng context key và dynamic data invalidation chưa hoàn chỉnh;
- texture pooling: chỉ là exact-match free-list, chưa phải LRU hay memory manager;
- capability: đã có snapshot limits/features và cờ `INDIRECT_FIRST_INSTANCE`; capability tier và policy fallback vẫn chưa hoàn chỉnh;
- platform support: có nền tảng `wgpu` nhưng chưa hoàn tất integration theo từng platform.

## Đã thiết kế nhưng chưa implement

- generational handle allocator foundation; typed resource store integration vẫn chưa hoàn tất;
- resource registry API có insert/lookup/remove và version tracking nền tảng; ownership/private store vẫn chưa hoàn tất;
- logical graph flatten plan vÃ  explicit dependency ordering cÆ¡ báº£n; resource hazard, usage vÃ  pass compilation váº«n chÆ°a implement;
- compute/copy pass;
- transient resource allocator;
- submission tracker logic nền tảng; frame memory reuse an toàn theo submission vẫn chưa tích hợp;
- structured validation/error cÆ¡ báº£n cho graph/resource/target; pipeline layout, usage vÃ  dynamic offset validation váº«n cÃ²n;
- cross-backend test matrix;
- MSAA/resolve và attachment model đầy đủ hơn;
- indirect draw/dispatch.

## Design debt hiện tại

- surface format hard-code đã được loại bỏ khỏi compiler; lifecycle surface/resize/lost vẫn chưa hoàn chỉnh;
- state cache bind group cố định bốn slot;
- ring buffer wrap không đồng bộ với GPU in-flight;
- raw resource map public;
- target dimension bị lặp nhưng chưa validate;
- `unwrap()`/panic trong code liên quan library;
- readback giả định format RGBA bốn byte;
- examples/benchmarks đã được đồng bộ với texture registry và `RenderNodePool`; vẫn còn warning/style debt cần dọn riêng;
- tài liệu cũ tuyên bố hoàn thành vượt quá implementation.

## Chính sách rewrite

Không rewrite mù toàn bộ crate. Giữ lại experiment và visual fixture đang hoạt động làm reference, nhưng xem public API và resource/execution internals là provisional cho tới khi design document trong thư mục này được chấp nhận.
