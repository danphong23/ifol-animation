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
- [TC19](../../tests/reports/tc19_audio_viz_report.md): một pass audio spectrum
  với 16 frequency bands và một draw command; vision/structural, validation và
  cache parity đạt, raw còn khác 7 byte ở 7 pixel với sai số tối đa `1/255`,
  nên là `ĐẠT CÓ ĐIỀU KIỆN`. Web cold cao do lazy shader/pipeline compilation;
  warm còn `3.5 ms`.
- [TC20](../../tests/reports/tc20_perspective_report.md): một pass sprite
  perspective 2.5D với MVP matrix cố định và một draw command; vision/structural,
  validation, cache và raw parity đều đạt tuyệt đối (`0` byte khác).
- [TC21](../../tests/reports/tc21_masking_report.md): một pass SDF circular mask
  cho avatar crop với một draw command; vision/structural, validation và cache
  parity đạt, raw còn khác 1 byte ở 1 pixel với sai số tối đa `1/255`, nên là
  `ĐẠT CÓ ĐIỀU KIỆN`.
- [TC22](../../tests/reports/tc22_particles_instanced_report.md): một pass
  hardware instancing với 100 instance trong một draw command; integer hash giúp
  placement/scale/rotation xác định, vision/structural, validation, cache và raw
  parity đều đạt tuyệt đối (`0` byte khác).
- [TC23](../../tests/reports/tc23_color_replace_report.md): một pass HSV palette
  replacement với một draw command; vision/structural, validation, cache và raw
  parity đều đạt tuyệt đối (`0` byte khác).
- [TC24](../../tests/reports/tc24_distortion_mesh_report.md): một pass vertex
  wind/sway deformation với một draw command; vision/structural, validation,
  cache và raw parity đều đạt tuyệt đối (`0` byte khác).
- [TC25](../../tests/reports/tc25_shadow_rimlight_report.md): một pass
  rimlight/drop-shadow với hai instance trong một draw command; vision/structural,
  validation, cache và raw parity đều đạt tuyệt đối (`0` byte khác).
- [TC26](../../tests/reports/tc26_glitch_report.md): một pass deterministic
  glitch/RGB split với một draw command; integer hash loại bỏ sai khác backend,
  vision/structural, validation, cache và raw parity đều đạt tuyệt đối (`0` byte
  khác).
- [TC27](../../tests/reports/tc27_godrays_report.md): một pass 100-sample radial
  godrays trên nền rừng; vision/structural, validation và cache parity đạt, raw
  còn khác 33 byte ở 33 pixel với sai số tối đa `1/255`, nên là `ĐẠT CÓ ĐIỀU
  KIỆN`.
- [TC28](../../tests/reports/tc28_ripple_report.md): một pass radial ripple
  trên city PNG canonical; vision/structural, validation và cache parity đạt,
  raw còn khác 15 byte ở 15 pixel với sai số tối đa `1/255`, nên là `ĐẠT CÓ
  ĐIỀU KIỆN`.
- [TC29](../../tests/reports/tc29_crt_vhs_report.md): một pass CRT/VHS với
  barrel distortion, scanlines, vignette, RGB split và integer-hash noise;
  vision/structural, validation và cache parity đạt, raw còn khác 591 byte ở
  588 pixel với sai số tối đa `1/255`, nên là `ĐẠT CÓ ĐIỀU KIỆN`.
- [TC30](../../tests/reports/tc30_dissolve_report.md): hai pass chroma key →
  dissolve/burn trên sprite sheet và noise map PNG canonical; vision/structural,
  validation, cache và raw parity đạt tuyệt đối (`0` byte khác).
- [TC31](../../tests/reports/tc31_light_sweep_report.md): hai pass chroma key →
  light sweep trên mage PNG canonical; vision/structural, validation, cache và
  raw parity đạt tuyệt đối (`0` byte khác).
- [TC32](../../tests/reports/tc32_page_curl_report.md): ba pass scene A → scene
  B → page curl với dual texture; vision/structural, validation và cache đạt,
  raw còn khác 3 byte ở 3 pixel với sai số tối đa `1/255`, nên là `ĐẠT CÓ
  ĐIỀU KIỆN`.
- [TC33](../../tests/reports/tc33_pixelation_report.md): hai pass chroma key →
  pixelation 16px trên paladin PNG canonical; vision/structural, validation,
  cache và raw parity đạt tuyệt đối (`0` byte khác).
- [TC34](../../tests/reports/tc34_directional_blur_report.md): hai pass chroma
  key → directional blur 30 độ trên mage PNG canonical; vision/structural,
  validation, cache và raw parity đạt tuyệt đối (`0` byte khác).
- [TC35](../../tests/reports/tc35_halftone_report.md): hai pass chroma key →
  halftone 45 độ trên paladin PNG canonical; vision/structural, validation và
  cache parity đạt. Raw còn khác 6 byte ở 2 pixel, sai số tối đa `1/255`, nên
  là `ĐẠT CÓ ĐIỀU KIỆN`.
- [TC36](../../tests/reports/tc36_radial_blur_report.md): hai pass chroma key →
  radial zoom blur 30 mẫu trên paladin PNG canonical; vision/structural,
  validation, cache và raw parity đạt tuyệt đối (`0` byte khác).
- [TC37](../../tests/reports/tc37_chromatic_aberration_report.md): hai pass
  chroma key → RGB split xuyên tâm trên paladin PNG canonical; vision/structural,
  validation, cache và raw parity đạt tuyệt đối (`0` byte khác).
- [TC38](../../tests/reports/tc38_kaleidoscope_report.md): hai pass chroma key
  → kaleidoscope sáu phân đoạn trên mage PNG canonical; vision/structural,
  validation và cache đạt, raw khác 63 byte ở 47 pixel, tối đa `1/255`, nên là
  `ĐẠT CÓ ĐIỀU KIỆN`.
- [TC39](../../tests/reports/tc39_scanlines_report.md): hai pass chroma key →
  hologram scanlines 200 dòng trên mage PNG canonical; vision/structural,
  validation và cache đạt, raw khác 5 byte ở 5 pixel, tối đa `1/255`, nên là
  `ĐẠT CÓ ĐIỀU KIỆN`.
- [TC40](../../tests/reports/tc40_vignette_grain_report.md) đến
  [TC49](../../tests/reports/tc49_trim_paths_report.md): batch 10 TC độc lập
  dùng manifest/graph fingerprint riêng, input PNG canonical và shader WGSL
  dùng chung giữa Desktop/Web; 10/10 pass validation và vision. TC43, TC46,
  TC47, TC49 đạt raw byte parity tuyệt đối. TC40, TC41, TC42, TC44, TC45 và
  TC48 đạt có điều kiện; số byte/pixel khác và max delta được ghi trong từng
  report. TC49 đã sửa padding uniform để loại sai lệch ABI WebGPU.

Phạm vi cache cần hiểu thống nhất: `ifol-gpu` có transient resource pools và
render-bundle reuse theo graph/resource, nhưng hiện không cung cấp API xóa
cache driver/GPU. Test Desktop tạo harness mới theo TC; Web hủy texture/buffer
của TC sau khi chạy. Hai cách này cô lập resource logic, còn pipeline cache của
driver/browser vẫn nằm ngoài quyền kiểm soát của `ifol-gpu`; do đó timing cold
trong các report không được diễn giải thành cold start tuyệt đối.

- [TC50](../../tests/reports/tc50_exposure_inspector_report.md) đến
  [TC52](../../tests/reports/tc52_soft_particles_report.md): 3 TC advanced
  dùng manifest/fingerprint chung giữa Desktop/Web, input PNG canonical và
  output/report riêng. Cả 3 pass validation, cold/warm và vision. TC50 khác
  96 byte, TC51 khác 163 byte, TC52 khác 4495 byte; đều được phân loại
  `ĐẠT CÓ ĐIỀU KIỆN` vì khác biệt raw còn lại thuộc pixel/backend sau khi đã
  loại lỗi graph/ABI.

- [TC53](../../tests/reports/tc53_blend_modes_report.md) đến
  [TC55](../../tests/reports/tc55_dual_kawase_report.md): 3 TC advanced dùng
  manifest/fingerprint chung giữa Desktop/Web, shader WGSL dùng chung và
  input PNG canonical. Cả 3 pass validation, cold/warm output ổn định và
  vision. TC53 khác 69 byte/51 pixel, max delta `1/255`; TC54 khác 976
  byte/350 pixel, max delta `71/255`; TC55 đạt raw byte parity tuyệt đối.
  TC53–TC54 là `ĐẠT CÓ ĐIỀU KIỆN`, TC55 là `ĐẠT`. TC53 phải đổi
  `textureSample` sang `textureSampleLevel` vì WebGPU yêu cầu sample trong
   uniform control flow; đây là sửa portability shader, không phải chấp nhận
   output sai.

- [TC56](../../tests/reports/tc56_dynamic_resize_report.md) đến
  [TC58](../../tests/reports/tc58_mrt_gbuffer_report.md): 3 TC kiểm thử
  lifecycle target, depth-stencil và MRT bằng manifest/fingerprint chung,
  input PNG canonical và runner Desktop/Web riêng. Cả 3 pass validation,
  cold/warm output ổn định và vision. TC56 khác 5200 byte/1979 pixel, max
  delta `37/255`; TC57 khác 836 byte/296 pixel, max delta `139/255`; TC58
  đạt raw byte parity tuyệt đối. TC56–TC57 là `ĐẠT CÓ ĐIỀU KIỆN`, TC58 là
  `ĐẠT`. TC56 ban đầu có lỗi scale do runner Desktop dùng UV ratio thay vì
  kích thước pixel thật; đã sửa và chạy lại cả hai môi trường trước khi
  phân loại kết quả.

- [TC59](../../tests/reports/tc59_sampler_modes_report.md) đến
  [TC61](../../tests/reports/tc61_compute_buffer_math_report.md): 3 TC đã
  chuyển khỏi graph JSON tự ghi sang manifest canonical và runner Desktop/Web
  chung. Cả 3 pass validation, vision và cold/warm; fingerprint manifest lần
  lượt là `41c657787fe74841`, `861e27bfb471246e` và `91a37c1c43c4f64c`.
  TC59 khác 24752 byte/16675 pixel, max delta `3/255`; TC60 khác 6165
  byte/4032 pixel, max delta `5/255`; TC61 khác 52 byte/49 pixel, max delta
  `1/255`. TC59–TC61 là `ĐẠT CÓ ĐIỀU KIỆN` ở raw parity; TC61 vẫn đạt kiểm
  chứng số học Desktop `10240/10240`, max diff `0.00005054 < 1e-4`, còn Web
  pass compute dispatch và validation không lỗi. TC60 đã bổ sung clear đầu
  chu kỳ đầu để warm run không phụ thuộc dữ liệu còn trong target pong.

- [TC62](../../tests/reports/tc62_storage_texture_report.md) đến
  [TC64](../../tests/reports/tc64_audio_fft_report.md): 3 TC compute đã
  chuyển sang runner Desktop/Web chung và manifest canonical. Cả 3 pass
  validation, vision và cold/warm; fingerprint lần lượt là
  `29f38bc13430eb96`, `49c5ea09d42ea7cb` và `eb63136e435ed1cb`.
  TC62 khác 59987 byte/31132 pixel, max delta `128/255`; TC63 khác 204777
  byte/74419 pixel, max delta `222/255`; TC64 khác 3893 byte/2088 pixel,
  max delta `99/255`. Đây là `ĐẠT CÓ ĐIỀU KIỆN` về raw parity; vision xác
  nhận cấu trúc đúng mô tả. TC63 đã reset toàn bộ particle buffer trước warm,
  tránh đo state nối tiếp giữa hai lần chạy.

- [TC65](../../tests/reports/tc65_workgroup_blur_report.md) đến
  [TC67](../../tests/reports/tc67_pingpong_report.md): 3 TC compute đã
  chuyển sang manifest canonical và runner Desktop/Web chung. Fingerprint lần
  lượt là `9219b57bf1c71f6b`, `52de157767d72d36` và `92b7444c45f8deee`; cả 3
  pass validation, vision và cold/warm. TC65 khác 56353 byte/43747 pixel,
  max delta `7/255`; TC66 khác 74771 byte/54379 pixel, max delta `242/255`
  nhưng numeric histogram đạt `480000/480000`; TC67 khác 8095 byte/6380
  pixel, max delta `10/255`. Cả 3 là `ĐẠT CÓ ĐIỀU KIỆN` về raw parity.
  TC67 giữ semantics runner cũ với 2480 bước và reset seed trước warm.

TC02 và TC03 được đánh dấu `ĐẠT CÓ ĐIỀU KIỆN`, không phải pixel-perfect. PNG
canonical được dùng như input fixture để loại decoder JPG khác nhau khỏi phép
đo; canonical export thực sự vẫn phải do higher layer quản lý theo
[canonical render và media output contract](../00-foundation/18-canonical-render-and-media-output-contract.md).
