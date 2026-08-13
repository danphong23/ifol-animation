# IFOL GPU: context-aware bundle cache

## Vấn đề

`RenderBundle` là object gắn với device/context đã tạo nó. Logical graph node
có thể được dùng lại cho nhiều viewport hoặc nhiều device, nhưng bundle không
được dùng chéo giữa các context.

## Contract

Host tạo executor với identity ổn định:

```text
RenderGraphExecutor::with_context_key(context_key)
```

`context_key` là token do host quản lý cho cặp device/viewport hoặc bất kỳ
execution context nào có lifetime bundle riêng. Core không tự hash pointer của
`wgpu::Device`, không đọc identity backend và không quản lý registry ownership.

Bundle key hiện bao gồm context key, sample count, attachment formats, pipeline/
bind-group/mesh versions và dynamic offsets. Hai context key khác nhau luôn
compile bundle riêng.

Executor mặc định dùng key `0` để giữ compatibility cho host chỉ có một context.
Host nhiều context phải cấp key khác nhau và không được tái sử dụng key trong khi
bundle cũ còn sống.

## Giới hạn

Đây là cache partitioning contract, chưa phải cache manager nhiều device hoàn
chỉnh. Host vẫn phải quản lý pool, registry và lifetime của context; pipeline
layout signature/reflection vẫn là metadata riêng.

## Test gate

Unit test chứng minh cache key khác nhau theo context key và sample count; toàn
bộ execution/benchmark gate tiếp tục chạy serialized.
