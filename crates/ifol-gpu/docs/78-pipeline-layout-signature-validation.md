# IFOL GPU: pipeline-layout signature validation

## Mục tiêu

Pipeline và bind group của `wgpu` là object opaque; core không thể dựa vào
getter giả định để so sánh layout. Host có thể khai báo contract bằng:

```text
PipelineLayoutResourceDescriptor {
    bind_group_layout_signatures: Vec<Option<u64>>,
}
BindGroupResourceDescriptor {
    layout_signature,
    dynamic_offset_count,
    dynamic_offset_alignment,
}
```

Signature tại cùng một slot phải giống nhau. Khi pipeline có metadata và command
gắn bind group, validation trước submit sẽ từ chối:

- bind group thiếu layout metadata trong khi pipeline yêu cầu;
- signature khác nhau tại slot;
- dynamic offset count/alignment không hợp lệ.

Đăng ký pipeline/bind group qua compatibility API cũ sẽ xóa metadata tương ứng,
tránh giữ descriptor cũ cho object mới.

## Tính chất mở rộng

`u64` chỉ là signature do host định nghĩa, không phải hash mà core tự suy ra từ
shader. Tầng reflection/pipeline builder sau này có thể tạo signature ổn định từ
layout thật mà không đổi graph command model.

## Phạm vi chưa tuyên bố

Signature không tự chứng minh binding type, visibility, min binding size hoặc
shader reflection. Những kiểm tra đó cần descriptor/reflection chi tiết hơn và
là task riêng. Khi thiếu metadata, core vẫn chỉ kiểm tra resource tồn tại và
slot device limit.

## Test gate

Test dynamic compute binding chứng minh path hợp lệ, lệch alignment và signature
layout mismatch đều trả kết quả typed trước execute.
