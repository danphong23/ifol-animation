# IFOL GPU: Memory, synchronization và cache

## Frame context

Mỗi submission thuộc một execution context rõ ràng. Context sở hữu:

- upload allocation;
- dynamic uniform/storage allocation;
- transient resource;
- command encoder state;
- submission identity và completion tracking.

Host không được bắt buộc gọi `reset()` không an toàn trong khi GPU vẫn có thể tham chiếu allocation cũ.

`SubmissionTracker` hiện cung cấp identity monotonic và trạng thái completion tối
thiểu ở tầng CPU. Nó là primitive để frame allocator hỏi `can_reuse_after`; nó chưa
tự kết nối với callback/poll của `wgpu` và không được xem là fence GPU hoàn chỉnh.

## Ring buffer

Ring buffer hiện align allocation và từ chối implicit wrap khi hết contiguous space, nên không tự ghi đè allocation cũ. Điều này loại bỏ một lỗi correctness rõ ràng nhưng chưa thay thế frame/submission-aware allocator. Implementation mục tiêu vẫn dùng frame segment, fence/submission completion hoặc allocator từ chối reuse cho tới khi GPU hoàn thành submission liên quan.

Allocation failure phải rõ ràng. Alignment, size overflow và dynamic-offset limit phải được validate.

## Upload và readback

Upload là phần riêng với semantics của render graph. Readback phải rõ ràng và bất đồng bộ khi có thể. Readback API phải mang theo texture format/aspect, không được giả định mọi texture đều là RGBA8 bốn byte.

Image encoding và filesystem output là tiện ích tùy chọn, không phải trách nhiệm bắt buộc của core device API.

## Transient resource

Transient texture/buffer reuse dựa trên descriptor và lifetime. Descriptor phải chứa mọi thuộc tính ảnh hưởng compatibility. Chỉ được reuse sau khi GPU không còn in-flight use trước đó.

## Cache invalidation

Compiled artifact và bundle phải quan sát resource version. Mutate/replace pipeline, bind group layout, attachment format, mesh binding hoặc resource usage liên quan phải invalidate artifact phụ thuộc. Boolean dirty flag đơn lẻ không đủ cho multi-viewport dùng chung.

## Tuyên bố hiệu năng

Không được hứa “một submission”, “zero allocation”, “O(1) rendering” hay FPS cố định. Đây là benchmark result hoặc policy của workload cụ thể, không phải API guarantee.
