# IFOL GPU: Mô hình command và pipeline

## Command

Command nên gần với `wgpu`, rõ ràng và compact. `DrawAction::Indexed` và `DrawAction::Procedural` hiện là subset đầu tiên hợp lệ. Mô hình sau này cần bao phủ:

- indexed và non-indexed draw;
- instanced draw;
- indirect draw;
- vertex/index buffer binding;
- bind group binding với dynamic offset;
- compute dispatch và indirect dispatch;
- copy và clear.

Command tham chiếu typed handle. Command không chứa entity, scene object, file path hoặc khái niệm animation.

## Pipeline

Pipeline là tổ hợp immutable của shader stage, pipeline layout, target format, primitive state, depth/stencil state, multisampling và fixed-function state liên quan. Blend mode thuộc pipeline, không thuộc draw command.

Core có thể expose low-level pipeline creation hoặc nhận `wgpu` pipeline đã tạo sẵn, nhưng ownership/lifetime phải nhất quán. Pipeline compatibility với pass attachment phải được validate.

## Chính sách shader

WGSL là ranh giới shader portable. Core không cung cấp material system hoặc shader graph. Core có thể cung cấp shader module creation, diagnostics và source cache tùy chọn.

Mệnh đề cũ rằng mọi shader chỉ output `@location(0)` là quá hạn chế cho 3D và deferred rendering. Cần hỗ trợ multiple color target, depth output, compute storage resource và vertex output tùy theo pipeline descriptor.

## Chính sách binding

Core không quy định semantic global/material/entity cho bind group. Đó là convention của tầng trên. Core phải dùng bind-group limit thật của device, không dùng array cố định, và validate mọi slot trước khi encode.

## Chính sách bundle

Render bundle là optimization, không phải source of truth. Bundle phải được cache ngoài logical graph node và key theo mọi thuộc tính ảnh hưởng đến validity, gồm target format, depth state, sample count, pipeline/resource version và bundle mode. Command có dynamic offset hoặc dynamic state theo frame không được bake sai vào bundle dùng lại.
