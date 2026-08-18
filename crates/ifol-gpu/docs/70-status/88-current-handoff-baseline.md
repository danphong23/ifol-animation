# Baseline và điểm bàn giao hiện tại

Tài liệu này dùng để bắt đầu chat/task mới mà không mất ý định hoặc trạng thái.

## Trạng thái kiểm tra gần nhất

Đã xác nhận trong workspace:

```text
cargo check -p ifol-gpu              PASS
cargo test -p ifol-gpu --lib        114 passed, 0 failed
cargo test -p ifol-gpu --no-default-features --lib 114 passed, 0 failed
cargo check -p ifol-gpu --examples --benches PASS (default features)
cargo test -p ifol-gpu --test tc_parity_canonical PASS
cargo test -p ifol-gpu --tests -- --test-threads=1 PASS (full desktop suite)
cargo check -p ifol-gpu --no-default-features --tests --examples --benches PASS
```

Full desktop regression suite đã chạy pass với 0 failed; TC95 hiện không có
target trong repository. Dự án còn có bộ test desktop đến TC105 và WebGPU runner.
Canonical parity probe
Desktop/Web đã tạo raw `Rgba8Unorm` giống nhau từng byte; xem
`tests/reports/webgpu_verification_report.md` và
`docs/70-status/89-upgrade-regression-and-parity.md`. Kết quả này chưa chứng
minh pixel parity cho toàn bộ TC98–TC105 hoặc mọi platform.

## Đã có và cần giữ lại

- graph dependency, hazard, cycle và nested flatten;
- render, compute, copy, indirect execution;
- resource descriptor validation;
- generational handle và version invalidation;
- submission-safe lifetime, deferred destruction và ring buffer;
- async readback và typed errors;
- extension registration, validation và fail-closed dispatch;
- desktop/WebGPU test evidence hiện có.

## Ghi chú cấu trúc hiện tại

- execution regression suite đã được tách khỏi execution facade theo các
  boundary validation, encoder, target, copy, compute và execution order;
- `src/graph/tests.rs` chứa graph regression suite; `src/graph/mod.rs` hiện
  là production facade nhỏ với `RenderTarget` và các re-export public;
- `src/extensions/tests.rs` chứa extension regression suite; extension
  facade hiện giữ contracts, validation và registries;
- `src/memory/frame_tests.rs` chứa frame lifecycle regression suite; frame
  production module chỉ giữ lifecycle logic và public API; legacy
  `FrameContext::seal` đã được loại bỏ, mọi consumer dùng
  `seal_with_deferred_textures`;
- `src/memory/lru_tests.rs` chứa transient pool regression suite;
  `src/memory/texture_pool.rs` giữ texture descriptor key và
  `TransientTexturePool`, còn `src/memory/buffer_pool.rs` giữ buffer
  descriptor key và `TransientBufferPool`; `lru_cache.rs` chỉ là facade
  re-export nhỏ để giữ canonical public path;
- `src/memory/ring_tests.rs` chứa ring buffer regression suite; ring buffer
  production module chỉ giữ allocation và submission-gated reset logic;
- `src/memory/submission_tests.rs` chứa submission tracker regression suite;
  production module chỉ giữ submission identity và completion tracking;
- `src/memory/deferred_tests.rs` chứa deferred destruction regression suite;
  production module chỉ giữ queue và completion-gated drain contract;
- `src/lib_tests.rs` chứa headless initialization regression test; root
  `lib.rs` chỉ giữ crate module declarations và public facade;
- `src/api/profiling_tests.rs` chứa timestamp profiling regression suite;
  `api/profiling.rs` chỉ giữ profiling contract và pool behavior;
- `src/execution/validation_errors.rs` chứa typed validation error contract;
  `validation.rs` giữ orchestration và các re-export nội bộ cần cho validation;
- `src/execution/validation_copy.rs` chứa copy-command và buffer-range
  validation helpers;
- `src/execution/validation_indirect.rs` chứa indirect-buffer range và usage
  validation; `validation.rs` giữ re-export nội bộ cho command validators;
- `src/execution/validation_texture.rs` chứa texture-copy, mip/aspect và
  format validation helpers;
- `src/execution/validation_target.rs` chứa render-target và depth/stencil
  validation helpers; `validation.rs` giữ graph-validation orchestration;
- `src/execution/validation_layout.rs` chứa bind-group slot, dynamic-offset
  và render/compute pipeline-layout validation helpers;
- `src/execution/validation_node.rs` chứa node/resource/command traversal
  validation; command-specific checks được delegate sang các validation command
  modules; `validation.rs` hiện là facade/orchestrator nhỏ;
- `src/execution/encoder_tests.rs` chứa regression tests cho bind-group slot và
  compute/draw/copy encoder;
- `src/execution/executor_contract_tests.rs` chứa extension-dispatch,
  profiling-entrypoint và execution-report regression tests;
- `src/execution/validation_contract_tests.rs` chứa preflight/validation
  contract tests cho target lookup, indirect range, texture aspect và bundle
  key;
- `src/execution/target_tests.rs` chứa target-size, MSAA resolve/depth
  validation và target execution regression tests;
- `src/execution/command_validation_tests.rs` chứa command/resource usage và
  buffer-range validation regression tests;
- `src/execution/copy_execution_tests.rs` chứa buffer/texture copy execution,
  copy validation và interleaved copy/draw segment regression tests;
- `src/execution/compute_execution_tests.rs` chứa compute-only execution và
  nested-compute ordering regression tests;
- `src/execution/dynamic_offset_tests.rs` chứa descriptor-aware dynamic-offset
  validation regression test;
- `src/execution/attachment_validation_tests.rs` chứa depth-attachment
  validation regression tests;
- `src/execution/pipeline_layout_validation_tests.rs` chứa render/compute
  pipeline-layout validation regression tests;
- `src/execution/indexed_resource_validation_tests.rs` chứa indexed/indirect
  mesh và buffer lookup validation regression tests;
- `src/execution/execution_order_tests.rs` chứa empty-graph, nested-subgraph và
  interleaved draw/copy/compute execution regression tests;
- `src/resources/registry_version_tests.rs` chứa version regression suite;
  `registry_descriptor_tests.rs` chứa descriptor validation suite;
  `registry_ownership_tests.rs` chứa owned-texture/deferred-destruction suite;
  `registry.rs` chỉ giữ state container và version API;
- `src/resources/versions.rs` hiện giữ cả version storage và version behavior;
  registry facade chỉ giữ container/constructor;
- Internal crate, examples, tests và benchmark code đã dùng canonical
  `resources::*` exports; registry implementation không còn public qua nested
  module path;
- `src/api/compatibility.rs` đã được loại bỏ; `api/mod.rs` chỉ còn profiling
  functionality và explicit canonical re-exports;
- `src/render/` compatibility facade đã được loại bỏ; render implementation
  canonical nằm trong `execution/render_pass.rs` và `execution/render_bundles.rs`;
- `src/extensions/validation.rs` chứa validation resource-usage của extension;
  `extensions/mod.rs` giữ contract/context/errors; `extensions/registry.rs`
  giữ extension và dispatcher registry state/lookup;
- `src/graph/flatten.rs` chứa `FlatRenderPlan`, `FlatRenderNode`, dependency và
  flatten error types; `graph/mod.rs` giữ public graph exports;
- `src/graph/flattening.rs` chứa `RenderGraph::flatten` traversal, nested graph
  path collection và flattened dependency/hazard ordering; `graph.rs` chỉ giữ
  graph model và construction API;
- `src/graph/ordering.rs` chứa dependency/hazard ordering của graph;
  `RenderGraph::effective_resource_usages` là internal helper dùng chung;
- `src/graph/resource_usage.rs` chứa resource declaration/accessor API của
  `RenderGraph`; storage hiện được mở ở mức `pub(crate)` cho các graph modules;
- `src/graph/commands.rs` chỉ là facade re-export; `draw_command.rs`,
  `compute_command.rs` và `copy_command.rs` giữ riêng từng command model và
  builder API;
- `src/graph/effective_usage.rs` chứa effective usage derivation từ copy/draw/
  compute commands và render target; `resource_usage.rs` chỉ giữ declaration
  và accessor API;
- `src/execution/executor.rs` chứa public `RenderGraphExecutor` facade và
  validation/execution-report orchestration; `src/execution/executor_profiling.rs`
  chứa các profiling entrypoints của executor;
- `src/execution/report.rs` chứa `ExecutionReport`, `ProfiledExecution` và
  `RenderGraphProfilingError`; `execution/mod.rs` giữ public re-export và
  module wiring;
- `src/execution/counts.rs` chứa execution diagnostics counting và recursive
  declared-usage counting;
- `src/execution/targets.rs` chứa target view resolution cho screen, offscreen
  và MSAA; compiler/nested compile dùng chung `TargetViews` internal contract;
- `src/execution/flat_compile.rs` chứa flat graph compilation, owner-path
  resolution và flatten-error mapping;
- `src/execution/nested_compile.rs` chứa nested graph compilation bottom-up;
- `src/execution/render_bundles.rs` chứa bundle update và render-node
  preparation;
- `src/execution/bundle_key.rs` chứa pure bundle cache identity calculation;
  `render_bundles.rs` giữ bundle encode/update lifecycle;
- `src/execution/render_pass.rs` chứa render pass lifecycle và graph pass
  traversal; `src/execution/draw.rs` chứa draw/bind-group/mesh/indirect
  command encoding; `render.rs` facade đã được loại bỏ;
- `src/execution/non_render.rs` chứa execution của extension/copy/compute không
  có render target;
- `src/execution/prepass.rs` chứa prepass extension/copy rồi compute;
- `src/execution/target_segments.rs` chứa ordered target segment render passes;
- `src/execution/validation_render.rs` chứa validation của draw commands;
- `src/execution/validation_compute.rs` chứa validation của compute commands;
- `src/execution/validation_copy.rs` chứa validation của copy commands cùng các
  buffer-range helpers; indirect validation nằm trong `validation_indirect.rs`
  và texture validation nằm trong `validation_texture.rs`;
- `src/backend/readback_ticket.rs` chứa mapping, submission wait và row-padding
  resolution của `ReadbackTicket`; `backend/readback.rs` giữ facade/API bắt đầu
  readback; `src/backend/readback_tests.rs` giữ readback regression suite;
- `src/graph/usage_overlap.rs` chứa resource access conflict và subresource/aspect
-  overlap facade; `src/graph/buffer_overlap.rs` giữ buffer-range matching và
  `src/graph/texture_overlap.rs` giữ texture/aspect matching; `graph/usage.rs`
  giữ usage types, range constructors và facade re-export;
- `src/graph/node_pool.rs` chứa `RenderNodePool` storage, allocation, mutation
  và removal; `graph/nodes.rs` giữ `RenderNode` model/behavior;
- `src/backend/builder_build.rs` chứa runtime adapter/device creation và surface
  configuration; `backend/builder.rs` giữ builder policy/configuration facade;
- builder/engine consumers nội bộ trong source, examples, benches, desktop tests
  và integration docs đã migrate từ `api::*` sang `backend::*`;
- compatibility facades cũ đã được loại bỏ khỏi source; các public domain modules
  hiện là canonical API. Alias execution `execute` và `execute_with_surface`
  cũng đã được migrate toàn bộ consumer sang các API `*_checked` và loại bỏ;
- `image` chỉ còn là dev-dependency phục vụ test/example support;
- production core không còn save/encode API hoặc `backend/texture_save.rs`;
- readback contract đã trả raw bytes kèm format qua `RawTextureReadback`; tuple
  readback API cũ đã được loại bỏ;
- canonical readback từ owned registry texture tự lấy format từ descriptor;
- Web runner có canonical offscreen parity probe và đo timing riêng;
- chưa có runtime matrix đầy đủ cho Metal, Linux, Android và iOS; browser mới
  có evidence cho WebGPU runner và canonical probe;
- canonical render/export path chưa thuộc `ifol-gpu`; asset decode, renderer
  deterministic và media encoder vẫn là responsibility của higher layer;
- PNG canonical trong `tests/shared_assets/textures/` chỉ là test fixture để
  giảm khác biệt decoder, không phải giới hạn format của core;
- canonical path đã được mô tả rõ trong
  [`18-canonical-render-and-media-output-contract.md`](../00-foundation/18-canonical-render-and-media-output-contract.md):
  tầng ngoài chuẩn hóa input và encode output; core chỉ execute và raw
  readback;
- quyết định boundary đã chốt: decoder, canonical asset bytes, color/alpha
  policy, media encoder và file output đều do tầng ngoài quản lý; không thêm
  API decode/encode hoặc nhánh PNG/JPEG/WebP/video vào `ifol-gpu`;
- canonical render/export là workflow do tầng ngoài sở hữu và điều phối. Tầng
  ngoài có thể dùng `ifol-gpu` làm execution backend, nhưng phải giữ quyền chọn
  canonical input, deterministic policy, readback verification và encoder;
- TC01, TC04, TC05, TC06, TC07 và TC08 đã đạt raw parity tuyệt đối; TC02 và
  TC03 đạt vision/structural/depth parity, còn TC08.5 đạt vision/structural
  parity. TC09 đạt raw parity tuyệt đối và xác nhận cold/warm output không đổi.
  TC10 đạt typed-error/no-panic contract và raw fallback parity tuyệt đối. TC08.5
  là case có điều kiện vì raw pixel parity còn sai khác màu/alpha. TC11 đã đạt
  raw parity tuyệt đối cho ba pass multi-viewport/compositor. TC12 đạt
  vision/structural parity nhưng còn 4 byte sai khác ở 4 pixel. Xem
  [chính sách parity](../00-foundation/19-cross-platform-parity-testing-policy.md)
  và các report TC tương ứng trong `tests/reports/`;
- TC13 đã đạt raw parity tuyệt đối cho graph 4 pass ping-pong Gaussian blur;
  Desktop/Web dùng cùng manifest fingerprint `4f37a8fd4102496e`, cùng 11 draw
  command và output cold/warm không đổi. Xem report TC13 trong
  `tests/reports/tc13_blur_report.md`;
- TC14 đã được chuẩn hóa thành graph 2 pass color grading/ACES Filmic với
  manifest fingerprint `f3824201007dd4a7`, cùng 8 draw command trên Desktop/Web.
  Vision/structural và cold/warm đạt; raw còn khác 18 byte ở 16 pixel, nên là
  `ĐẠT CÓ ĐIỀU KIỆN`. Xem `tests/reports/tc14_grading_report.md`;
- TC15 đã được chuẩn hóa thành graph 1 pass winter scene với manifest fingerprint
  `6ec7f347092fd77a`, cùng 7 draw command và 200 snow instances. Vision/
  structural, validation và cold/warm đạt; raw còn khác 73 byte ở 28 pixel,
  nên là `ĐẠT CÓ ĐIỀU KIỆN`. Xem `tests/reports/tc15_snow_report.md`;
- TC16 đã được chuẩn hóa thành graph 1 pass SDF với manifest fingerprint
  `8962fd4fa969ea29`, cùng 4 draw command và 4 procedural shapes. Texture dummy
  đã được loại bỏ khỏi test contract; shader chỉ nhận uniform ở group 0.
  Vision/structural, validation và cold/warm đạt; raw còn khác 1 byte ở 1 pixel,
  nên là `ĐẠT CÓ ĐIỀU KIỆN`. Xem `tests/reports/tc16_sdf_report.md`;
- TC17 đã được chuẩn hóa thành graph 2 pass outline/drop shadow với manifest
  fingerprint `bd209137e1b026dc`, cùng 5 draw command và 5 instances. Desktop /
  Web dùng chung manifest, vision/structural, validation và cold/warm đều đạt;
  raw còn khác 1 byte ở 1 pixel, sai số tối đa `1/255`, nên là `ĐẠT CÓ ĐIỀU
  KIỆN`. Xem `tests/reports/tc17_outline_report.md`;
- TC18 đã được chuẩn hóa thành graph 3 pass dual-texture glitch transition với
  manifest fingerprint `9c9b047f0733fa82`, cùng 5 draw command và 5 instances.
  Shader đã dùng integer hash để tránh khác biệt quyết định block do hàm
  `sin/fract` giữa backend; Desktop/Web đạt vision/structural, validation và
  cold/warm parity, raw còn khác 1 byte ở 1 pixel, sai số tối đa `1/255`, nên
  là `ĐẠT CÓ ĐIỀU KIỆN`. Xem `tests/reports/tc18_transition_report.md`;
- TC19 đã được chuẩn hóa thành graph 1 pass audio spectrum với manifest
  fingerprint `b214133532fd962a`, 1 draw command và 16 frequency bands đóng gói
  trong uniform. Desktop/Web đạt vision/structural, validation và cold/warm
  parity; raw còn khác 7 byte ở 7 pixel, sai số tối đa `1/255`, nên là `ĐẠT CÓ
  ĐIỀU KIỆN`. Web cold cao do lazy shader/pipeline compilation, warm còn
  `3.5 ms`. Xem `tests/reports/tc19_audio_viz_report.md`;
- TC20 đã được chuẩn hóa thành graph 1 pass perspective sprite với manifest
  fingerprint `990a63feb8c50405`, 1 draw command và MVP matrix cố định trong
  manifest. Desktop/Web đạt vision/structural, validation, cold/warm và raw
  parity tuyệt đối (`0` byte khác). Xem `tests/reports/tc20_perspective_report.md`;
- TC21 đã được chuẩn hóa thành graph 1 pass SDF circular mask với manifest
  fingerprint `c55143c9bb5e1cf1`, 1 draw command và avatar crop canonical.
  Desktop/Web đạt vision/structural, validation và cold/warm parity; raw còn
  khác 1 byte ở 1 pixel, sai số tối đa `1/255`, nên là `ĐẠT CÓ ĐIỀU KIỆN`.
  Xem `tests/reports/tc21_masking_report.md`;
- TC22 đã được chuẩn hóa thành graph 1 pass hardware instancing với manifest
  fingerprint `91868a1a00433fd4`, một draw command và 100 instance. Shader dùng
  integer hash cho placement/scale/rotation xác định giữa backend; Desktop/Web
  đạt vision/structural, validation, cold/warm và raw parity tuyệt đối (`0` byte
  khác). Xem `tests/reports/tc22_particles_instanced_report.md`;
- TC23 đã được chuẩn hóa thành graph 1 pass HSV palette replacement với manifest
  fingerprint `5e6dcfeb32712bc9`, một draw command và một instance. Desktop/Web
  dùng chung graph, đạt vision/structural, validation, cold/warm và raw parity
  tuyệt đối (`0` byte khác). Xem `tests/reports/tc23_color_replace_report.md`;
- TC24 đã được chuẩn hóa thành graph 1 pass vertex wind/sway deformation với
  manifest fingerprint `f409de1bc9146473`, một draw command và một instance.
  Desktop/Web dùng chung graph, đạt vision/structural, validation, cold/warm và
  raw parity tuyệt đối (`0` byte khác). Xem
  `tests/reports/tc24_distortion_mesh_report.md`;
- TC25 đã được chuẩn hóa thành graph 1 pass rimlight/drop-shadow với manifest
  fingerprint `15cf62a1e76cb4e2`, một draw command và hai instance. Desktop/Web
  dùng chung graph, đạt vision/structural, validation, cold/warm và raw parity
  tuyệt đối (`0` byte khác). Xem
  `tests/reports/tc25_shadow_rimlight_report.md`;
- TC26 đã được chuẩn hóa thành graph 1 pass deterministic glitch/RGB split với
  manifest fingerprint `9b01f0a73634c199`, một draw command và một instance.
  Shader dùng integer hash để giữ block selection đồng nhất giữa backend;
  Desktop/Web đạt vision/structural, validation, cold/warm và raw parity tuyệt
  đối (`0` byte khác). Xem `tests/reports/tc26_glitch_report.md`;
- TC27 đã được chuẩn hóa thành graph 1 pass 100-sample radial godrays với
  manifest fingerprint `39041e2d99fd185f`, một draw command và một instance.
  Desktop/Web đạt vision/structural, validation và cold/warm parity; raw còn
  khác 33 byte ở 33 pixel, sai số tối đa `1/255`, nên là `ĐẠT CÓ ĐIỀU KIỆN`.
  Xem `tests/reports/tc27_godrays_report.md`;
- TC28 đã được chuẩn hóa thành graph 1 pass radial ripple với manifest
  fingerprint `01228a36813398ce`, một draw command và một instance. Fixture
  JPEG city đã được materialize thành PNG canonical để Desktop/Web nạp cùng
  input bytes; vision/structural, validation, cache và raw parity đạt với 15
  byte khác ở 15 pixel, sai số tối đa `1/255`, nên là `ĐẠT CÓ ĐIỀU KIỆN`. Xem
  `tests/reports/tc28_ripple_report.md`;
- TC29 đã được chuẩn hóa thành graph 1 pass CRT/VHS với manifest fingerprint
  `a54801bc417a3b00`, một draw command và một instance. Desktop/Web dùng
  cùng PNG canonical và cùng graph; vision/structural, validation và cold/warm
  parity đạt. Raw còn khác 591 byte ở 588 pixel, sai số tối đa `1/255`, nên là
  `ĐẠT CÓ ĐIỀU KIỆN`. Xem `tests/reports/tc29_crt_vhs_report.md`;
- TC30 đã được chuẩn hóa thành graph 2 pass chroma key → dissolve/burn với
  manifest fingerprint `1c996b323cba7910`, hai draw command và hai instance.
  Desktop/Web dùng cùng hai PNG canonical; vision/structural, validation,
  cache và raw parity đạt tuyệt đối (`0` byte khác). Xem
  `tests/reports/tc30_dissolve_report.md`;
- TC31 đã được chuẩn hóa thành graph 2 pass chroma key → light sweep với
  manifest fingerprint `e8c707cfcbf0e9a7`, hai draw command và hai instance.
  Desktop/Web dùng cùng PNG canonical; vision/structural, validation, cache và
  raw parity đạt tuyệt đối (`0` byte khác). Xem
  `tests/reports/tc31_light_sweep_report.md`;
- TC32 đã được chuẩn hóa thành graph 3 pass scene A → scene B → page curl với
  manifest fingerprint `26faa4396e406466`, năm draw command và ba node. Shader
  page-curl dùng phép xấp xỉ toán học và ổn định hóa màu để giảm sai khác backend;
  vision/structural, validation và cache đạt, raw còn khác 3 byte ở 3 pixel với
  sai số tối đa `1/255`, nên là `ĐẠT CÓ ĐIỀU KIỆN`. Xem
  `tests/reports/tc32_page_curl_report.md`;
- TC33 đã được chuẩn hóa thành graph 2 pass chroma key → pixelation với manifest
  fingerprint `de9a0f7f14975043`, hai draw command và hai node. Desktop/Web
  dùng cùng PNG canonical; vision/structural, validation, cache và raw parity
  đạt tuyệt đối (`0` byte khác). Xem `tests/reports/tc33_pixelation_report.md`;
- TC34 đã được chuẩn hóa thành graph 2 pass chroma key → directional blur với
  manifest fingerprint `22559f51d1e1f5bf`, hai draw command và hai node.
  Desktop/Web dùng cùng PNG canonical; vision/structural, validation, cache và
  raw parity đạt tuyệt đối (`0` byte khác). Xem
  `tests/reports/tc34_directional_blur_report.md`;
- TC35 đã được chuẩn hóa thành graph 2 pass chroma key → halftone với manifest
  fingerprint `0bfdc815933931d8`, hai draw command và hai node. Desktop/Web
  dùng cùng PNG canonical; vision/structural, validation và cache parity đạt.
  Raw còn khác 6 byte ở 2 pixel, sai số tối đa `1/255`, nên là `ĐẠT CÓ ĐIỀU
  KIỆN`. Xem `tests/reports/tc35_halftone_report.md`;
- TC36 đã được chuẩn hóa thành graph 2 pass chroma key → radial blur với
  manifest fingerprint `e8635023d0c9c2fb`, hai draw command và hai node.
  Desktop/Web dùng cùng PNG canonical; vision/structural, validation, cache và
  raw parity đạt tuyệt đối (`0` byte khác). Xem
  `tests/reports/tc36_radial_blur_report.md`;
- TC37 đã được chuẩn hóa thành graph 2 pass chroma key → chromatic aberration
  với manifest fingerprint `7f5f010b70f54583`, hai draw command và hai node.
  Desktop/Web dùng cùng PNG canonical; vision/structural, validation, cache và
  raw parity đạt tuyệt đối (`0` byte khác). Xem
  `tests/reports/tc37_chromatic_aberration_report.md`;
- TC38 đã được chuẩn hóa thành graph 2 pass chroma key → kaleidoscope với
  manifest fingerprint `cf4713957e83abbf`, hai draw command và hai node.
  Desktop/Web dùng cùng PNG canonical; vision/structural, validation và cache
  parity đạt; raw còn khác 63 byte ở 47 pixel, sai số tối đa `1/255`, nên là
  `ĐẠT CÓ ĐIỀU KIỆN`. Xem `tests/reports/tc38_kaleidoscope_report.md`;
- TC39 đã được chuẩn hóa thành graph 2 pass chroma key → hologram scanlines
  với manifest fingerprint `5ea108ce90344f78`, hai draw command và hai node.
  Desktop/Web dùng cùng PNG canonical; vision/structural, validation và cache
  parity đạt; raw còn khác 5 byte ở 5 pixel, sai số tối đa `1/255`, nên là
  `ĐẠT CÓ ĐIỀU KIỆN`. Xem `tests/reports/tc39_scanlines_report.md`;
- TC40–TC49 đã được chuẩn hóa thành 10 manifest độc lập, mỗi TC có graph
  fingerprint riêng, runner Desktop/Web riêng và report tiếng Việt riêng.
  Cả 10/10 TC pass validation và vision; Desktop/Web dùng cùng manifest,
  input PNG canonical và shader WGSL tương ứng. Raw byte parity tuyệt đối đạt
  ở TC43, TC46, TC47 và TC49; các TC còn lại là `ĐẠT CÓ ĐIỀU KIỆN` với sai
  khác pixel được ghi cụ thể trong từng report `tests/reports/tc40_...` đến
  `tests/reports/tc49_...`;
- Batch TC40–TC49 được chạy lại tuần tự sau khi sửa portability WGSL và ABI
  uniform của TC49. Mỗi TC hủy resource logical sau khi hoàn tất. Không có API
  xóa cache driver/browser/GPU; vì vậy cold/warm trong report là cold/warm của
  lần execute sau khi device/pipeline đã tạo, không phải cold start tuyệt đối;
- Web runner có fallback metadata cô lập/cache thống nhất; report phải ghi rõ
  phạm vi này, không được gọi việc `destroy()` resource là xóa cache nền tảng;
- TC50–TC52 đã được chuyển sang manifest canonical và runner Desktop/Web riêng.
  Cả 3/3 pass validation, cold/warm output ổn định và vision đạt. Fingerprint
  lần lượt là `6eb21c3021072252`, `d8d597349c97b340` và
  `3f930de62616d52f`. Raw parity đều là `ĐẠT CÓ ĐIỀU KIỆN`; TC51 đã sửa
  lỗi Web uniform `key_color` làm lộ nền xanh, không còn sai lệch cấu trúc;
- TC53–TC55 đã được chuyển sang manifest canonical và runner Desktop/Web riêng.
  Cả 3/3 pass validation, cold/warm output ổn định và vision đạt. Fingerprint
  lần lượt là `0045bf536afcf57d`, `99296555552df541` và
  `2a88441e6a8ac270`. TC53 khác 69 byte/51 pixel với max delta `1/255`;
  TC54 khác 976 byte/350 pixel với max delta `71/255`; TC55 đạt raw parity
  tuyệt đối. Không còn runner hợp lệ nào tham chiếu graph legacy của TC53–TC55;
  các file `tests/graphs/tc53...tc55.json` đã được loại bỏ;
- TC53 yêu cầu `textureSampleLevel` trong shader dùng sample bên trong nhánh
  điều kiện để tương thích validation WebGPU; Desktop và Web dùng cùng WGSL
  sau khi sửa portability này;
- TC56–TC58 đã được chuyển sang manifest canonical và runner Desktop/Web riêng.
  Cả 3/3 pass validation, cold/warm output ổn định, fingerprint trùng và vision
  đạt. Fingerprint lần lượt là `712b3ac12833ff81`, `33c65cd0ace1f7da` và
  `99bc2711d6947215`. TC56 khác 5200 byte/1979 pixel với max delta `37/255`;
  TC57 khác 836 byte/296 pixel với max delta `139/255`; TC58 đạt raw byte parity
  tuyệt đối. TC56–TC57 là `ĐẠT CÓ ĐIỀU KIỆN`, TC58 là `ĐẠT`;
- TC56 ban đầu có lỗi scale crop ở runner Desktop do dùng trực tiếp tỉ lệ UV
  thay vì quy đổi theo aspect ratio của texture/target. Lỗi đã được sửa trong
  shared runner, sau đó Desktop và Web đều chạy lại trước khi phân loại kết quả;
- Không còn runner hợp lệ nào tham chiếu graph legacy của TC56–TC58; các file
  `tests/graphs/tc56...tc58.json` đã được loại bỏ. Mỗi TC vẫn tạo harness/device
  mới để cô lập test; `ifol-gpu` không tuyên bố có thể xóa cache driver/browser/GPU
  portable. Vì vậy cold/warm chỉ là timing của execute sau khi device/pipeline
  đã tạo, không phải cold start tuyệt đối;
- TC59–TC61 đã được chuyển sang manifest canonical và runner Desktop/Web chung.
  Fingerprint lần lượt là `41c657787fe74841`, `861e27bfb471246e` và
  `91a37c1c43c4f64c`; cả 3 pass validation/vision/cold-warm. Raw parity lần
  lượt khác 24752 byte, 6165 byte và 52 byte; các report ghi rõ pixel diff và
  phân loại `ĐẠT CÓ ĐIỀU KIỆN`. TC61 còn kiểm chứng 10240/10240 vec4 trên
  Desktop với max diff `0.00005054`; TC60 đã reset target pong ở đầu chu kỳ
  để hai lần chạy không phụ thuộc resource state cũ;
- Không còn runner hợp lệ nào tham chiếu graph legacy của TC59–TC60; các file
  `tests/graphs/tc59_sampler_modes.json` và `tc60_ping_pong.json` đã được loại
  bỏ. TC61 trước đây không có graph JSON canonical và hiện lấy manifest làm
  graph contract duy nhất;
- TC62–TC64 đã được chuyển sang manifest canonical và runner Desktop/Web chung.
  Fingerprint lần lượt là `29f38bc13430eb96`, `49c5ea09d42ea7cb` và
  `eb63136e435ed1cb`; cả 3 pass validation, vision và cold/warm. Raw parity
  lần lượt khác 59987 byte/31132 pixel, 204777 byte/74419 pixel và 3893
  byte/2088 pixel; các report phân loại cả 3 là `ĐẠT CÓ ĐIỀU KIỆN`. TC63 đã
  reset toàn bộ particle buffer trước warm để hai lượt đo không nối tiếp state.
- Không có graph JSON legacy cho TC62–TC64; manifest là graph contract duy nhất
  của batch này. Các output Web được lưu riêng để đối chiếu raw bytes và vision.
- TC65–TC67 đã được chuyển sang manifest canonical và runner Desktop/Web chung.
  Fingerprint lần lượt là `9219b57bf1c71f6b`, `52de157767d72d36` và
  `92b7444c45f8deee`; cả 3 pass validation, vision và cold/warm. TC65 khác
  56353 byte/43747 pixel, TC66 khác 74771 byte/54379 pixel và TC67 khác 8095
  byte/6380 pixel; report ghi rõ max delta và phân loại `ĐẠT CÓ ĐIỀU KIỆN`.
  TC66 đạt numeric histogram `480000/480000`; TC67 giữ 2480 bước như runner
  cũ và reset seed trước warm.
- Không có graph JSON legacy cho TC65–TC67; manifest là graph contract duy nhất
  của batch này. Các runner Desktop cũ đã được thay bằng wrapper gọi shared
  runner, Web catalog/card và output/report song song đã được bổ sung.
- file `docs/ifol-gpu-upgrade-plan.md` chưa phải execution plan đã cập nhật
  trạng thái.

## Trạng thái handoff

Đợt refactor incremental F21–F31 đã hoàn tất bounded audit. Các production
file còn dài hơn khoảng 180 dòng đều giữ một responsibility rõ ràng; riêng
`graph/tests.rs` là suite nhất quán cho graph scheduling/hazard và không có
boundary đủ độc lập để tách thêm. Không còn task tách file bắt buộc trong
baseline này; thay đổi tiếp theo chỉ nên mở task mới khi có requirement hoặc
behavior mới.

Benchmark target hiện compile sạch, không còn warning Rust trong crate; working
tree phải được kiểm tra lại trước mỗi task mới.

## Hợp đồng với chat/task mới

Chat/task mới phải:

1. đọc file này;
2. đọc `00-foundation/16-current-intent-and-refactor-workflow.md`;
3. đọc task đang chạy trong `17-incremental-module-splitting-plan.md`;
4. kiểm tra Git status để không đụng vào thay đổi ngoài scope;
5. làm đúng một task, test pass rồi commit;
6. cập nhật status/docs nếu contract hoặc baseline thay đổi.
