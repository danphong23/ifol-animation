# IFOL GPU: FrameContext và owned texture lifecycle

## Contract mới

`FrameContext::defer_owned_texture(registry, handle)` tách một
`OwnedTextureResource` khỏi registry nhưng giữ backing object trong frame
context. Texture không bị drop ngay sau khi handle bị gỡ khỏi registry.

Frame có texture đang chờ deferred phải được seal bằng:

```text
seal_with_deferred_textures(
    submission,
    transient_texture_pool,
    transient_buffer_pool,
    deferred_texture_queue,
)
```

Không còn API `seal` rút gọn; mọi frame đều dùng
`seal_with_deferred_textures`, truyền deferred queue tường minh để lifetime
contract không phụ thuộc vào compatibility path.

## Trình tự an toàn

```text
registry --defer_owned_texture--> FrameContext
                                   |
queue.submit --> seal_with_deferred_textures(submission)
                                   |
                                   v
                    DeferredDestructionQueue(last_use=submission)
                                   |
tracker.mark_completed --> drain_completed --> drop texture
```

`FrameContext::reset_after` vẫn chỉ mở frame sau completion. Queue deferred là
đối tượng host truyền vào và host chịu trách nhiệm drain; core không tự poll
GPU hoặc tự present surface.

## Invariants và test gate

- thiếu handle trả lỗi typed và không để lại handle giả trong pending set;
- seal mới đưa đúng một object vào queue;
- drain trước completion không trả object;
- drain sau completion trả object và frame có thể reset;
- test frame chạy serialized cùng full crate gate.
