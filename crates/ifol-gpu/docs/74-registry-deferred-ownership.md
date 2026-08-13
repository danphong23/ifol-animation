# IFOL GPU: registry và deferred ownership

## Vấn đề

Resource registry có thể tháo một `OwnedTextureResource` khỏi handle map trong
khi command buffer của submission trước vẫn đang dùng texture. Drop backing
object ngay sau khi remove là không an toàn.

## Contract

`ResourceRegistry::defer_owned_texture_destruction` thực hiện nguyên tử ở cấp
API:

1. tháo texture khỏi registry và invalid hóa các metadata/view liên quan;
2. đưa `OwnedTextureResource` vào `DeferredDestructionQueue` cùng
   `last_use: SubmissionId`;
3. trả `true` nếu handle tồn tại, `false` nếu không có owned texture.

Queue chỉ trả object qua `drain_completed(&SubmissionTracker)` sau khi
`last_use` đã hoàn tất. Host giữ queue sống đủ lâu và chịu trách nhiệm drain;
core không tự poll GPU.

## Invariant

- handle đã tháo không còn được executor lookup thấy;
- backing texture vẫn còn trong queue trước completion;
- completion sớm không làm queue trả object;
- completion đúng submission cho phép drain và drop an toàn;
- gọi lại với handle đã tháo không tạo queue entry giả.

Đường `remove_owned_texture` vẫn được giữ cho host đã tự quản lý lifetime.
