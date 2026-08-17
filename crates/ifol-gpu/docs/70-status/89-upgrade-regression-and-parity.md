# Báo cáo nâng cấp ifol-gpu: regression và parity

Ngày kiểm tra: 2026-08-17

## Kết quả

- Full desktop regression suite: PASS, 0 failed.
- Các test số TC01–TC105 hiện có trong repository đều pass; repository không có
  target `TC95`. Suite cũng chạy canonical offscreen parity probe.
- Unit tests: 114 passed với default features và 114 passed với
  `--no-default-features`.
- `cargo check -p ifol-gpu --tests --examples --benches`: PASS.
- `cargo check -p ifol-gpu --no-default-features --tests --examples --benches`:
  PASS.
- `git diff --check`: PASS sau khi loại whitespace phát sinh trong report.

## Canonical Desktop/Web output

Probe dùng cùng graph, cùng clear value `[0.03, 0.04, 0.07, 1.0]`, kích thước
`800x600` và format `Rgba8Unorm`. Raw readback Desktop/Web có cùng kích thước
`1,920,000` bytes, `different_bytes=0`, `max_byte_delta=0`.

SHA-256 của cả hai raw output:

```text
4F2AB7130334569606F07A9F0304A2A39DDFCC89C2563B54F8B4384777C813E2
```

Timing quan sát được trong môi trường kiểm thử:

| Path | Render time |
| --- | ---: |
| Desktop native | 1.5224 ms |
| WebGPU browser | 23.90 ms |

Timing chỉ có ý nghĩa so sánh trong cùng môi trường chạy; nó không phải
benchmark giữa mọi GPU/OS/browser.

## Phạm vi kết luận

Canonical offscreen path đã chứng minh contract raw output giống nhau từng byte
giữa Desktop và WebGPU. Điều này chưa chứng minh pixel parity của mọi graph,
mọi shader, mọi surface format hoặc mọi backend native. Các report test case
không được gắn nhãn pixel-perfect Web nếu chưa có probe riêng cho test case đó.

Production core hiện không encode PNG/JPEG và không phụ thuộc windowing; output
file, browser surface và platform color policy thuộc lớp bên ngoài. `ifol-gpu`
chỉ nhận resource/format contract, thực thi graph/shader/pipeline và trả raw
readback khi caller yêu cầu.

Đây là boundary chủ động, không phải phần việc còn thiếu của đợt làm sạch:
decoder và canonical input thuộc tầng asset; render contract và raw readback
thuộc `ifol-gpu`; encoder và media file thuộc tầng export. Khi bổ sung JPEG,
PNG, WebP hoặc video ở sản phẩm, chỉ tầng ngoài được mở rộng.

## Chứng nhận theo test case

Các report canonical hiện tại:

- [TC01](../../tests/reports/tc01_report.md): raw parity tuyệt đối;
- [TC02](../../tests/reports/tc02_single_quad_report.md): vision và structural
  parity đạt, raw còn sai khác màu/pixel;
- [TC03](../../tests/reports/tc03_zbuffer_report.md): vision, structural và
  depth parity đạt, raw còn sai khác màu/alpha.
- [TC04](../../tests/reports/tc04_alpha_blend_report.md): vision, structural,
  depth và raw parity đạt tuyệt đối với fixture canonical.
- [TC05](../../tests/reports/tc05_interleaved_report.md): vision, structural và
  raw parity đạt tuyệt đối với chuỗi pass A→B→C và fixture canonical.
- [TC06](../../tests/reports/tc06_gc_report.md): pool invariant, vision,
  structural và raw parity đạt tuyệt đối với fixture canonical.
- [TC07](../../tests/reports/tc07_recursion_report.md): graph đệ quy 5 cấp,
  flatten order và raw parity đạt tuyệt đối với canonical crop/sampler contract.
- [TC08](../../tests/reports/tc08_massive_report.md): 2 draw command, 10.000
  procedural instances và raw parity đạt tuyệt đối với background canonical.
- [TC08.5](../../tests/reports/tc08_5_nightsky_report.md): vision/structural
  parity đạt; raw còn khác 1 byte ở 1 pixel, sai số kênh tối đa `1/255`.
- [TC09](../../tests/reports/tc09_caching_report.md): cold + 10 warm lần,
  output cache không đổi và raw parity đạt tuyệt đối.
- [TC10](../../tests/reports/tc10_fallback_report.md): typed error
  `MissingBindGroup(999999)` không panic, fallback magenta và raw parity đạt
  tuyệt đối; Web validation là contract mirror.
- [TC11](../../tests/reports/tc11_viewport_report.md): hai viewport 400x600,
  ba pass compositor và raw parity đạt tuyệt đối.
- [TC12](../../tests/reports/tc12_chroma_report.md): sky canonical và 5 sprite
  chroma-key; vision/structural parity đạt, raw còn khác 4 byte ở 4 pixel với
  sai số kênh tối đa `1/255`.
- [TC13](../../tests/reports/tc13_blur_report.md): bốn pass Gaussian blur
  ping-pong và depth of field; vision/structural parity, raw parity và
  cold/warm cache parity đạt tuyệt đối.
- [TC14](../../tests/reports/tc14_grading_report.md): hai pass color grading
  điện ảnh/ACES Filmic; vision/structural parity và cold/warm cache parity đạt,
  raw còn khác 18 byte ở 16 pixel với sai số tối đa `2/255`, nên là `ĐẠT CÓ
  ĐIỀU KIỆN`.
- [TC15](../../tests/reports/tc15_snow_report.md): một pass winter scene với
  200 snow instances; vision/structural, validation và cache parity đạt, raw
  còn khác 73 byte ở 28 pixel với sai số tối đa `24/255`, nên là `ĐẠT CÓ ĐIỀU
  KIỆN`.
- [TC16](../../tests/reports/tc16_sdf_report.md): một pass với bốn hình SDF
  procedural, không dùng texture; vision/structural, validation và cache parity
  đạt, raw còn khác 1 byte ở 1 pixel với sai số tối đa `1/255`, nên là `ĐẠT CÓ
  ĐIỀU KIỆN`.
- [TC17](../../tests/reports/tc17_outline_report.md): hai pass outline/drop
  shadow với layer trong suốt, sky và 5 draw instances; vision/structural,
  validation và cache parity đạt, raw còn khác 1 byte ở 1 pixel với sai số tối
  đa `1/255`, nên là `ĐẠT CÓ ĐIỀU KIỆN`.
- [TC18](../../tests/reports/tc18_transition_report.md): ba pass render hai cảnh
  rồi chuyển cảnh dual-texture glitch với 5 draw instances; vision/structural,
  validation và cache parity đạt, integer hash giúp loại bỏ sai khác block lớn
  giữa backend, raw còn khác 1 byte ở 1 pixel với sai số tối đa `1/255`, nên là
  `ĐẠT CÓ ĐIỀU KIỆN`.

TC02 và TC03 được đánh dấu `ĐẠT CÓ ĐIỀU KIỆN`, không phải pixel-perfect. PNG
canonical được dùng như input fixture để loại decoder JPG khác nhau khỏi phép
đo; canonical export thực sự vẫn phải do higher layer quản lý theo
[canonical render và media output contract](../00-foundation/18-canonical-render-and-media-output-contract.md).
