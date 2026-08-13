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

API `seal` cũ vẫn giữ tương thích cho frame chỉ có transient resource. Nếu gọi
nó trong frame còn owned texture pending, core trả
`FrameContextError::DeferredDestructionQueueRequired` để tránh lifetime sai.

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
- gọi `seal` cũ khi có pending owned texture không submit/làm mất ownership;
- seal mới đưa đúng một object vào queue;
- drain trước completion không trả object;
- drain sau completion trả object và frame có thể reset;
- test frame chạy serialized cùng full crate gate.
