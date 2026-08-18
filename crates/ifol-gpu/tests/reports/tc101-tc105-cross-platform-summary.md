# Tổng hợp đối chiếu Desktop/WebGPU: TC101–TC105

## Phạm vi

Các TC được chạy lại trên Desktop và WebGPU sau khi chuyển Web sang cùng đường canonical:

`graph → texture offscreen cố định format → raw readback → PNG canonical`

Ảnh canvas/presentation chỉ dùng làm preview, không dùng để kết luận parity.

## Kết quả

| TC | Format canonical | Desktop | Web cold / warm | Pixel khác nhau | Max delta | Kết luận |
|---|---|---:|---:|---:|---:|---|
| [TC101](tc101_texture_copy_report.md) | `Rgba8UnormSrgb` | 72,84 ms | 5,40 / 4,00 ms | 6 / 480.000 | 1/255 | Đạt |
| [TC102](tc102_buffer_copy_report.md) | `Rgba8UnormSrgb` | 50,85 ms | 5,80 / 3,20 ms | 1 / 480.000 | 1/255 | Đạt |
| [TC103](tc103_depth_aspect_copy_report.md) | `Rgba8UnormSrgb` | 45,01 ms | 5,40 / 3,80 ms | 0 | 0/255 | Exact |
| [TC104](tc104_extension_dispatch_report.md) | `Rgba8UnormSrgb` | 37,04 ms | 4,40 / 2,90 ms | 3 / 480.000 | 1/255 | Đạt |
| [TC105](tc105_pingpong_echo_report.md) | `Rgba8Unorm` | 114,02 ms | 4,80 / 3,10 ms | 19.663 / 480.000 | 5/255 | Đạt có giới hạn |

## Ảnh canonical

| TC | Desktop | WebGPU |
|---|---|---|
| TC101 | [PNG](../outputs/desktop/tc101_texture_copy.png) | [PNG](../outputs/web/tc101_texture_copy.png) |
| TC102 | [PNG](../outputs/desktop/tc102_buffer_copy.png) | [PNG](../outputs/web/tc102_buffer_copy.png) |
| TC103 | [PNG](../outputs/desktop/tc103_depth_aspect_copy.png) | [PNG](../outputs/web/tc103_depth_aspect_copy.png) |
| TC104 | [PNG](../outputs/desktop/tc104_extension_dispatch.png) | [PNG](../outputs/web/tc104_extension_dispatch.png) |
| TC105 | [PNG](../outputs/desktop/tc105_pingpong_echo.png) | [PNG](../outputs/web/tc105_pingpong_echo.png) |

## Đánh giá

- TC101–TC104 đạt parity gần như tuyệt đối; sai lệch chỉ 0–6 pixel, delta tối đa 1/255.
- TC105 đã loại bỏ sai lệch gamma/độ sáng lớn do canvas, đồng thời sửa alpha blending và sampler để khớp Desktop. Phần còn lại là sai số bounded của feedback sampling giữa backend; chưa tuyên bố bit-exact.
- TC104 chứng minh parity output. Implementation extension thật là `ExtensionDispatchRegistry` trên Desktop; Web dùng fallback mô phỏng CommandBuffer.
- Baseline cũ TC71–TC73 vẫn giữ: TC71 lệch 15 pixel, max delta 2; TC72 và TC73 exact.

**Commit:** `f41845e test(gpu): canonicalize TC101-TC105 web parity`
