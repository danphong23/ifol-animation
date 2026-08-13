# IFOL GPU: texture copy theo aspect

## API

`CopyCommand::texture_to_texture` giữ behavior cũ với aspect `All`.
`CopyCommand::texture_to_texture_aspect` cho phép chọn `All`, `DepthOnly` hoặc
`StencilOnly`; `with_texture_mips` áp dụng cho cả hai variant.

Compiler chuyển aspect của graph sang `wgpu::TextureAspect` khi encode copy.
Validation kiểm tra format nguồn và đích có hỗ trợ aspect được yêu cầu trước
submit.

## Graph hazard

Copy aspect-aware tạo `TextureAspectRange` cho source read và destination write.
Do đó copy depth và copy stencil trên cùng mip/layer không tự tạo hazard với
nhau, trong khi `All` vẫn overlap cả hai aspect.

## Compatibility

Command constructor cũ không thay đổi. Variant mới chỉ cần dùng khi host thật
sự copy depth/stencil aspect; host vẫn phải cấp texture descriptor có usage
`COPY_SRC`/`COPY_DST` phù hợp.

## Test gate

Unit test bảo vệ format–aspect compatibility; graph hazard tests bảo vệ aspect
overlap/disjoint. Các texture copy runtime cũ tiếp tục chạy qua full regression.

