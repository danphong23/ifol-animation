# ifol-gpu Architecture Upgrade Plan — backlog dài hạn

> **Trạng thái:** Đây là tài liệu target/backlog dài hạn do AI khác tạo. Nó
> không phải execution plan hiện hành và không được chạy tuần tự nguyên khối.
> Execution plan hiện tại là
> [`00-foundation/17-incremental-module-splitting-plan.md`](00-foundation/17-incremental-module-splitting-plan.md),
> còn điểm bàn giao là
> [`70-status/88-current-handoff-baseline.md`](70-status/88-current-handoff-baseline.md).
>
> Mọi task trong file này phải được đối chiếu với source và status hiện tại
> trước khi thực hiện. Không lặp lại phần graph/resource đã đạt chỉ vì chúng
> vẫn xuất hiện trong backlog.

> **Mục tiêu:** nâng cấp `ifol-gpu` thành GPU core sạch, generic, ổn định, dễ mở rộng và dễ bảo trì.
>
> **Nguyên tắc bắt buộc:** mỗi task là một thay đổi nhỏ. **Làm → test → PASS → commit → task tiếp theo.** Nếu FAIL thì không commit thay đổi lỗi; sửa và test lại.

---

## 0. Architectural Target

`ifol-gpu` là **GPU execution substrate**, không phải rendering/domain engine.

### Core được phép biết

- GPU adapter/device/queue
- features/limits/capabilities
- GPU resource và resource lifetime
- texture/buffer/sampler/bind group/pipeline
- render/compute/copy execution
- synchronization/hazard
- render graph mechanics
- GPU memory
- backend/platform mechanics

### Core không được biết

- animation / scene / ECS
- asset semantics
- image/video codec
- PNG/JPEG export
- color management / color science
- ACES / BT.709 / P3 / PQ / HLG semantics
- material/effect/domain semantics
- application policy

### Policy vs mechanism

```text
Host / higher layer
    │
    ├── policy
    │     ├── backend preference
    │     ├── color policy
    │     ├── asset policy
    │     └── domain semantics
    │
    ▼
ifol-gpu
    └── mechanism
         ├── resources
         ├── graph
         ├── execution
         ├── memory
         └── GPU capabilities
              │
              ▼
             wgpu
```

### Color boundary

`ifol-gpu` chỉ hiểu **physical GPU format**, ví dụ `wgpu::TextureFormat`.

Color semantics nằm ở higher layer:

```text
ColorSpace
TransferFunction
Primaries
ColorTransform
HDR policy
Tone mapping
Color grading
```

Không thêm các abstraction này vào `ifol-gpu` chỉ vì tầng trên đang cần.

---

# 1. Workflow Rules

## 1.1. Task loop

Mỗi task phải theo:

```text
Inspect
  ↓
Smallest coherent change
  ↓
cargo fmt
  ↓
cargo check
  ↓
Relevant tests
  ↓
Regression tests
  ↓
PASS?
 ├─ NO → fix → test again
 └─ YES → commit → next task
```

Không:
- gom nhiều task rồi mới test;
- commit code đang fail;
- bỏ/sửa test để che bug;
- chuyển phase khi phase chưa đạt DoD;
- refactor ngoài scope task.

## 1.2. Commit rule

Format:

```text
gpu(<scope>): <short description>
```

Ví dụ:

```text
docs(gpu): define architecture boundaries
gpu(deps): remove image from runtime core
gpu(readback): keep texture readback format-neutral
resources: stabilize resource handles
graph: isolate validation
execution: isolate command encoding
```

Docs-only:

```text
docs(gpu): ...
```

## 1.3. Validation levels

### Level A

```bash
cargo fmt --all -- --check
cargo check -p ifol-gpu
```

### Level B

```bash
cargo test -p ifol-gpu
```

### Level C

```bash
cargo test --workspace
```

### Level D

```bash
cargo clippy -p ifol-gpu --all-targets --all-features -- -D warnings
```

Nếu baseline hiện tại chưa sạch với `-D warnings`, ghi nhận baseline trước; không sửa ngoài scope.

### Level E

GPU integration/backend tests liên quan:
- render
- compute
- resources
- graph
- synchronization
- memory
- readback
- surface
- cross-backend nếu môi trường hỗ trợ

---

# 2. Target Architecture

```text
crates/ifol-gpu/
└── src/
    ├── lib.rs
    ├── backend/
    │   ├── mod.rs
    │   ├── builder.rs
    │   ├── engine.rs
    │   ├── capabilities.rs
    │   ├── surface.rs
    │   └── readback.rs
    ├── resources/
    │   ├── mod.rs
    │   ├── registry.rs
    │   ├── handle.rs
    │   ├── descriptor.rs
    │   └── resource.rs
    ├── graph/
    │   ├── mod.rs
    │   ├── graph.rs
    │   ├── node.rs
    │   ├── command.rs
    │   ├── target.rs
    │   ├── dependency.rs
    │   ├── compiler.rs
    │   └── validation.rs
    ├── execution/
    │   ├── mod.rs
    │   ├── executor.rs
    │   ├── encoder.rs
    │   ├── render_pass.rs
    │   ├── compute_pass.rs
    │   ├── bindings.rs
    │   ├── pipeline.rs
    │   └── synchronization.rs
    ├── memory/
    │   ├── mod.rs
    │   ├── allocator.rs
    │   ├── ring_buffer.rs
    │   ├── deferred.rs
    │   └── cache.rs
    └── extensions/
        ├── mod.rs
        └── ...
```

> Đây là target. Không tạo hàng loạt file chỉ vì target; chỉ tách khi responsibility thực sự độc lập.

---

# PHASE 0 — Architecture Contract

## Task 0.1 — Audit public API

**Làm**
- đọc `lib.rs`;
- liệt kê `pub mod`, `pub use`;
- xác định API thật sự cần public;
- xác định implementation detail đang vô tình public.

**Test**

```bash
cargo check -p ifol-gpu
cargo test -p ifol-gpu
```

**Commit**

```text
docs(gpu): audit public api surface
```

## Task 0.2 — Architecture boundary document

Tạo:

```text
docs/architecture/ifol-gpu-boundaries.md
```

Ghi:
- responsibilities;
- forbidden dependencies;
- policy vs mechanism;
- color boundary;
- dependency direction.

**Test**

```bash
cargo check -p ifol-gpu
```

**Commit**

```text
docs(gpu): define architecture boundaries
```

## Task 0.3 — Dependency direction matrix

Định nghĩa module nào được phụ thuộc module nào.

**Test**

```bash
cargo check -p ifol-gpu
```

**Commit**

```text
docs(gpu): define module dependency rules
```

## Task 0.4 — Baseline

Ghi nhận:
- fmt;
- check;
- test;
- clippy;
- test count;
- known failures;
- backend/platform đang test được.

Tạo:

```text
docs/architecture/ifol-gpu-baseline.md
```

**Commit**

```text
docs(gpu): record architecture baseline
```

### Phase 0 DoD

- [ ] Boundary document
- [ ] Dependency direction
- [ ] Color ownership
- [ ] Public API baseline
- [ ] Test baseline

---

# PHASE 1 — Dependency & Domain Leakage Cleanup

## Task 1.1 — Audit runtime dependencies

Kiểm tra đặc biệt:

```text
wgpu
winit
image
bytemuck
log
pollster
thiserror
```

Với mỗi dependency: runtime core cần hay chỉ test/example?

**Test**

```bash
cargo check -p ifol-gpu
cargo test -p ifol-gpu
```

**Commit**

```text
docs(deps): classify ifol-gpu dependencies
```

## Task 1.2 — Remove `image` runtime dependency nếu chỉ phục vụ export/test

Chuyển sang dev/test utility nếu phù hợp.

**Test**

```bash
cargo check -p ifol-gpu
cargo test -p ifol-gpu
cargo test --workspace
```

**Commit**

```text
gpu(deps): remove image from runtime core
```

## Task 1.3 — Remove file/image encoding khỏi `GpuEngine`

Core chỉ:

```text
Texture → raw readback bytes
```

Không:

```text
Texture → PNG/JPEG/file
```

**Test**
- readback tests;
- texture format tests;
- GPU regression.

**Commit**

```text
gpu(readback): keep texture readback format-neutral
```

## Task 1.4 — Audit `Rgba8UnormSrgb`

Tìm toàn bộ occurrence và phân loại:
- production;
- test;
- example;
- fixture;
- higher-layer policy.

Production core không tự quyết sRGB nếu không có lý do architectural.

**Test**

```bash
cargo test -p ifol-gpu
cargo test --workspace
```

**Commit**

```text
gpu(format): remove inappropriate srgb policy from core
```

## Task 1.5 — Audit `winit`

Nếu chỉ phục vụ windowing policy:
- đưa khỏi runtime core;
- surface abstraction không phụ thuộc windowing policy.

**Test**

```bash
cargo check -p ifol-gpu
cargo test --workspace
```

**Commit**

```text
gpu(surface): decouple core from windowing policy
```

### Phase 1 DoD

- [ ] Không còn image/file encoding trong core.
- [ ] Không còn color policy hard-code không cần thiết.
- [ ] Runtime dependency sạch.
- [ ] Existing tests PASS.

---

# PHASE 2 — Public API & Module Boundary

## Task 2.1 — Stable public facade

Hướng tới:

```rust
pub use backend::GpuEngine;
pub use graph::RenderGraph;
pub use resources::{TextureHandle, /* ... */};

// internal modules private where possible
```

**Test**

```bash
cargo check --workspace
cargo test --workspace
```

**Commit**

```text
api(gpu): define stable public facade
```

## Task 2.2 — Remove accidental internal coupling

Tìm workspace imports như:

```text
ifol_gpu::execution::...
ifol_gpu::memory::...
```

thay bằng public facade khi phù hợp.

**Test**

```bash
cargo check --workspace
```

**Commit**

```text
api(gpu): remove internal module coupling
```

## Task 2.3 — Protect API boundaries

Thêm compile-level/docs tests nếu hữu ích.

**Test**

```bash
cargo test --workspace
```

**Commit**

```text
test(gpu): protect public api boundaries
```

### Phase 2 DoD

- [ ] Public API nhỏ, có chủ đích.
- [ ] Downstream không phụ thuộc implementation detail.
- [ ] Nội bộ có thể refactor mà không phá public API.

---

# PHASE 3 — Resource Architecture

## Task 3.1 — Separate descriptors

Tách:

```text
resources/descriptor.rs
resources/registry.rs
```

Descriptor chỉ mô tả GPU resource.

**Test**

```bash
cargo test -p ifol-gpu
```

**Commit**

```text
resources: separate descriptors from registry
```

## Task 3.2 — Stabilize handles

Định nghĩa rõ:
- identity;
- generation/version nếu có;
- invalid handle;
- typed vs generic handle.

**Test**
- create/destroy;
- stale handle;
- duplicate handle;
- lookup.

**Commit**

```text
resources: stabilize resource handles
```

## Task 3.3 — Isolate registry

Registry chỉ:
- identity;
- lookup;
- lifetime;
- ownership.

Không execution logic.

**Test**

```bash
cargo test -p ifol-gpu
cargo test --workspace
```

**Commit**

```text
resources: isolate registry responsibilities
```

## Task 3.4 — Define lifetime state machine

```text
Created → Alive → Referenced → Submitted → Retired → Destroyed
```

Enforce invariant phù hợp.

**Test**
- lifetime;
- deferred destruction;
- memory pressure.

**Commit**

```text
resources: document and enforce lifetime invariants
```

## Task 3.5 — Split registry implementation

Chỉ tách file sau khi responsibility rõ.

**Test sau extraction.**

**Commit**

```text
resources: split registry implementation
```

### Phase 3 DoD

- [ ] Descriptor không chứa domain semantics.
- [ ] Handle contract rõ.
- [ ] Registry không làm execution.
- [ ] Lifetime invariant rõ.
- [ ] Resource tests PASS.

---

# PHASE 4 — Graph Architecture

## Task 4.1 — Audit graph responsibilities

Phân loại:
- graph model;
- node;
- command;
- target;
- dependency;
- validation;
- compilation;
- execution leakage.

**Test**

```bash
cargo test -p ifol-gpu
```

**Commit**

```text
docs(graph): classify graph responsibilities
```

## Task 4.2 — Extract graph model

Target:

```text
graph/graph.rs
```

Graph chỉ giữ topology/state.

**Test**
- graph creation;
- add/remove node;
- lifecycle.

**Commit**

```text
graph: isolate graph model
```

## Task 4.3 — Extract commands

Target:

```text
graph/command.rs
```

Commands mô tả GPU operations, không encode trực tiếp.

**Test**
- command construction;
- graph execution regression.

**Commit**

```text
graph: isolate command model
```

## Task 4.4 — Extract validation

Target:

```text
graph/validation.rs
```

Validation:
- resource conflicts;
- invalid graph;
- missing dependency;
- invalid usage;
- cycle.

**Test**
- invalid graph;
- cycle;
- hazard.

**Commit**

```text
graph: isolate validation
```

## Task 4.5 — Extract dependency analysis

Target:

```text
graph/dependency.rs
```

Bao gồm:
- dependency edges;
- topological order;
- cycle detection.

**Test**
- DAG;
- diamond;
- disconnected graph;
- cycle;
- deterministic ordering.

**Commit**

```text
graph: isolate dependency analysis
```

## Task 4.6 — Introduce `CompiledGraph`

Pipeline:

```text
RenderGraph
    ↓
Validate
    ↓
Analyze dependencies
    ↓
Compile
    ↓
CompiledGraph
```

**Test**
- compiled ordering;
- deterministic compilation where required.

**Commit**

```text
graph: introduce compiled graph
```

### Phase 4 DoD

- [ ] Graph không encode GPU commands trực tiếp.
- [ ] Validation độc lập.
- [ ] Dependency analysis độc lập.
- [ ] CompiledGraph là input rõ ràng cho executor.
- [ ] Graph/hazard tests PASS.

---

# PHASE 5 — Execution Architecture

## Task 5.1 — Extract executor

Target:

```text
execution/executor.rs
```

Input:

```text
CompiledGraph
```

Output:

```text
GPU submission
```

**Test**
- render;
- compute;
- graph integration.

**Commit**

```text
execution: introduce graph executor
```

## Task 5.2 — Extract command encoder

```text
execution/encoder.rs
```

Chỉ encoding.

**Test**
- render;
- compute;
- copy;
- clear.

**Commit**

```text
execution: isolate command encoding
```

## Task 5.3 — Extract render pass

```text
execution/render_pass.rs
```

**Test**
- render;
- attachments;
- multisample/depth nếu có.

**Commit**

```text
execution: isolate render pass encoding
```

## Task 5.4 — Extract compute pass

```text
execution/compute_pass.rs
```

**Test**
- dispatch;
- storage buffer;
- storage texture;
- bounds safety.

**Commit**

```text
execution: isolate compute pass encoding
```

## Task 5.5 — Extract bindings

```text
execution/bindings.rs
```

**Test**
- bind group;
- layout;
- dynamic resources;
- invalid bindings.

**Commit**

```text
execution: isolate binding mechanics
```

## Task 5.6 — Extract pipeline subsystem

```text
execution/pipeline.rs
```

Target:

```text
PipelineDescriptor
    ↓
PipelineKey
    ↓
PipelineCache
```

**Test**
- cache hit/miss;
- invalidation;
- render/compute pipeline.

**Commit**

```text
execution: isolate pipeline cache
```

## Task 5.7 — Extract synchronization

```text
execution/synchronization.rs
```

Chịu trách nhiệm:
- hazard tracking;
- ordering;
- submission synchronization.

**Test**
- ping-pong;
- read-after-write;
- write-after-read;
- write-after-write;
- multi-pass hazards.

**Commit**

```text
execution: isolate synchronization
```

## Task 5.8 — Finalize execution module

Chỉ khi các extraction PASS:
- `execution/mod.rs` trở thành facade;
- loại bỏ implementation dump.

**Test**

```bash
cargo test -p ifol-gpu
cargo test --workspace
cargo clippy -p ifol-gpu --all-targets --all-features -- -D warnings
```

**Commit**

```text
execution: finalize modular execution architecture
```

### Phase 5 DoD

- [ ] `execution/mod.rs` chỉ là facade.
- [ ] Không còn file >100 KB.
- [ ] Graph không biết execution implementation.
- [ ] Executor nhận CompiledGraph.
- [ ] Render/compute/bindings/pipeline/sync độc lập.
- [ ] Full regression PASS.

---

# PHASE 6 — Backend / Surface / Capability

## Task 6.1 — Stabilize `GpuEngineBuilder`

Giữ:
- backend preference;
- power preference;
- features;
- limits;
- fallback policy.

Không thêm application policy.

**Test**
- adapter selection;
- fallback;
- required features;
- limits.

**Commit**

```text
backend: stabilize engine builder policy boundary
```

## Task 6.2 — Extract surface management

```text
backend/surface.rs
```

**Test**
- configure;
- resize;
- reconfigure;
- lost/outdated handling nếu có.

**Commit**

```text
backend: isolate surface management
```

## Task 6.3 — Formalize capabilities

```text
backend/capabilities.rs
```

Expose:
- supported formats;
- features;
- limits;
- surface capabilities.

**Test**
- capability queries;
- unsupported feature behavior.

**Commit**

```text
backend: formalize capability model
```

## Task 6.4 — Extract readback

```text
backend/readback.rs
```

Readback phải format-aware nhưng **color-policy-neutral**.

**Test**
- formats;
- row alignment;
- dimensions;
- async ticket lifecycle.

**Commit**

```text
backend: isolate texture readback
```

### Phase 6 DoD

- [ ] Core không phụ thuộc window policy nếu không cần.
- [ ] Backend behavior capability-driven.
- [ ] Surface management độc lập.
- [ ] Readback độc lập.
- [ ] Không platform hard-code không cần thiết.

---

# PHASE 7 — Memory Architecture

## Task 7.1 — Audit memory responsibilities

Phân loại:
- allocation;
- deferred destruction;
- frame lifecycle;
- ring buffer;
- cache;
- submission.

**Commit**

```text
docs(memory): define memory responsibilities
```

## Task 7.2 — Extract allocator

```text
memory/allocator.rs
```

**Test**
- allocation;
- reuse;
- free;
- pressure.

**Commit**

```text
memory: isolate allocator
```

## Task 7.3 — Extract deferred destruction

```text
memory/deferred.rs
```

**Test**
- submit;
- retire;
- destroy;
- pressure.

**Commit**

```text
memory: isolate deferred destruction
```

## Task 7.4 — Extract ring buffer

```text
memory/ring_buffer.rs
```

**Test**
- wrap;
- alignment;
- overflow;
- frame reuse.

**Commit**

```text
memory: isolate ring buffer
```

## Task 7.5 — Extract cache

```text
memory/cache.rs
```

**Test**
- hit;
- miss;
- eviction;
- lifetime.

**Commit**

```text
memory: isolate GPU cache mechanics
```

### Phase 7 DoD

- [ ] Memory responsibilities độc lập.
- [ ] Lifetime/synchronization không bị trộn với registry.
- [ ] Memory pressure tests PASS.

---

# PHASE 8 — Test Architecture & Portability Certification

## Task 8.1 — Organize tests

Target:

```text
tests/
├── unit/
├── resources/
├── graph/
├── execution/
├── memory/
├── backend/
└── integration/
```

Chỉ tổ chức, không đổi semantics.

**Test**

```bash
cargo test --workspace
```

**Commit**

```text
test(gpu): organize tests by subsystem
```

## Task 8.2 — Graph invariant tests

Thêm:
- deterministic dependency ordering;
- cycle detection;
- invalid resource usage;
- unused resource;
- conflicting write.

**Commit**

```text
test(graph): add graph invariants
```

## Task 8.3 — Resource invariant tests

Thêm:
- stale handle;
- double destroy;
- invalid resource;
- lifetime ordering.

**Commit**

```text
test(resources): add resource invariants
```

## Task 8.4 — Execution invariant tests

Thêm:
- render/compute isolation;
- submission ordering;
- pipeline cache;
- binding validation;
- synchronization.

**Commit**

```text
test(execution): add execution invariants
```

## Task 8.5 — Backend matrix

Nếu CI/environment cho phép:

```text
Vulkan
DX12
Metal
WebGPU
```

cùng logical tests.

**Commit**

```text
ci(gpu): add backend validation matrix
```

## Task 8.6 — Regression gate

CI gate:

```text
fmt
check
test
clippy
workspace test
GPU integration
```

**Commit**

```text
ci(gpu): enforce architecture regression gate
```

### Phase 8 DoD

- [ ] Test phân lớp.
- [ ] Graph/resource/execution invariants được kiểm thử.
- [ ] CI regression gate.
- [ ] Cross-backend matrix khả thi.

---

# PHASE 9 — Extension Architecture

## Task 9.1 — Define extension contract

Extension có thể:
- declare GPU resources;
- declare pipelines;
- declare graph operations;
- provide shader implementation.

Extension không được:
- sửa internal executor;
- phụ thuộc implementation details;
- kéo domain dependency vào core.

**Commit**

```text
docs(extensions): define extension contract
```

## Task 9.2 — Audit current extensions

Phân loại:
- core-worthy;
- higher-layer;
- test-only;
- domain-specific.

**Commit**

```text
docs(extensions): classify existing extensions
```

## Task 9.3 — Move domain-specific implementations upward

Các phần như:
- image;
- video;
- YUV conversion;
- color;
- effects;

nếu không phải GPU primitive thì chuyển ra higher layer.

**Test từng migration.**

**Commit**

```text
extensions: move domain semantics out of gpu core
```

## Task 9.4 — Stabilize extension API

**Test**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

**Commit**

```text
api(extensions): stabilize extension boundary
```

### Phase 9 DoD

- [ ] Extension không làm core phình.
- [ ] Domain-specific code nằm đúng layer.
- [ ] Extension API ổn định.

---

# PHASE 10 — Final Architecture Audit

## Task 10.1 — Dependency audit

Không còn:
- domain dependency;
- cyclic dependency;
- accidental public coupling.

**Commit**

```text
audit(gpu): verify dependency boundaries
```

## Task 10.2 — File-size audit

Guideline:

```text
mod.rs          < 5 KB
normal module   < 15 KB preferred
complex module  < 25 KB
>30 KB          review required
>50 KB          exceptional
>100 KB         prohibited
```

Không tách file máy móc; tách theo responsibility.

**Commit**

```text
audit(gpu): enforce module size guidelines
```

## Task 10.3 — Public API audit

Kiểm tra:
- unnecessary exports;
- unstable types;
- implementation leakage.

**Commit**

```text
audit(gpu): finalize public api
```

## Task 10.4 — Domain leakage audit

Search:

```text
image
video
color
animation
scene
material
asset
ACES
BT.709
P3
PQ
HLG
PNG
JPEG
```

Mỗi occurrence phải được phân loại.

**Commit**

```text
audit(gpu): verify domain isolation
```

## Task 10.5 — Final regression

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Cộng GPU/backend integration tests.

Chỉ khi tất cả PASS mới tạo release milestone:

```text
release(gpu): complete architecture modernization
```

---

# 3. Color System — OUTSIDE `ifol-gpu`

Sau khi core boundary ổn định, xây color system ở higher layer:

```text
ifol-color/
├── color_space.rs
├── primaries.rs
├── transfer.rs
├── transform.rs
├── hdr.rs
└── display.rs
```

Shader layer:

```text
shader/color/
├── yuv_to_rgb
├── linearize
├── encode
├── gamut_map
├── tone_map
└── grade
```

Pipeline:

```text
Asset / Media
      ↓
Color metadata
      ↓
Color system
      ↓
Color transform
      ↓
RenderGraph
      ↓
ifol-gpu
      ↓
GPU
```

`ifol-gpu` chỉ nhận resource/format/commands.

---

# 4. Definition of "ifol-gpu hoàn thiện"

## Architecture

- [ ] Domain-independent.
- [ ] Policy/mechanism tách rõ.
- [ ] Graph/Execution tách rõ.
- [ ] Resource/Lifetime tách rõ.
- [ ] Backend/Platform tách rõ.
- [ ] Memory tách rõ.
- [ ] Extension boundary rõ.

## Code quality

- [ ] Không còn God File.
- [ ] `mod.rs` là facade.
- [ ] Public API nhỏ.
- [ ] Internal API có thể refactor.
- [ ] Responsibility từng module rõ.

## Portability

- [ ] Không platform hard-code.
- [ ] Capability-driven.
- [ ] Surface abstraction sạch.
- [ ] Cross-backend tests.

## Color

- [ ] Không color-management semantics trong core.
- [ ] GPU format không bị nhầm với color meaning.
- [ ] Color system ở higher layer.

## Testing

- [ ] Unit tests.
- [ ] Graph invariants.
- [ ] Resource invariants.
- [ ] Execution invariants.
- [ ] Memory tests.
- [ ] Backend tests.
- [ ] Regression suite.
- [ ] Cross-platform tests.

---

# 5. Final Architecture

```text
┌─────────────────────────────────────────────────────────┐
│                    IFOL APPLICATION                      │
│ animation / scene / asset / editor / media              │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                 DOMAIN / RENDER LAYERS                  │
│ color / effects / materials / shaders / media           │
│ domain semantics / policy                               │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                     RENDER GRAPH                        │
│ WHAT should happen                                      │
│ resource dependencies / pass topology                   │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                       IFOL-GPU                          │
│ Resources / Graph execution / Command encoding          │
│ Synchronization / Memory / Pipeline / Capabilities      │
│ Surface mechanics / Readback                             │
│                                                         │
│                NO DOMAIN SEMANTICS                      │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                         WGPU                            │
├───────────────┬───────────────┬───────────────┬─────────┤
│ Vulkan        │ DX12          │ Metal         │ WebGPU  │
└───────────────┴───────────────┴───────────────┴─────────┘
```

---

# 6. Golden Rule

> **Không thêm abstraction vào `ifol-gpu` chỉ vì tầng trên đang cần nó.**
>
> Trước mỗi feature mới, hỏi:
>
> **"Đây là GPU mechanism hay application/domain policy?"**
>
> - GPU mechanism → `ifol-gpu`
> - domain policy → higher layer
> - color semantics → color layer
> - media semantics → media layer
> - shader algorithm → shader/extension layer
> - platform preference → host
