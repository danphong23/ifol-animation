# Feature inventory của `ifol-gpu`

Đây là inventory ngắn gọn của implementation hiện tại, không phải tuyên bố
“V1.0 production”. API public đang ở dòng `0.1.x`; xem
[crate README](README.md) và [API reference](docs/60-guides/94-guide-public-api-reference.md).

## Core hiện có

- `GpuEngineBuilder`: chọn adapter/backend, feature, limit, power preference
  và surface tùy chọn.
- `ResourceRegistry`: typed handle, descriptor validation, layout metadata,
  owned texture, version invalidation và deferred ownership.
- `RenderGraph`: draw, compute, copy, subgraph, extension, explicit
  dependency và resource usage/hazard analysis.
- `RenderGraphExecutor`: validate trước submit, flatten graph, render/compute/
  copy/indirect execution, MSAA resolve, depth/stencil và surface execution.
- `memory`: submission tracker, frame context, transient pools, ring buffer và
  deferred destruction.
- `RawTextureReadback`: synchronous và ticket-based raw readback với format
  contract.
- `extensions`: host-provided extension registration/dispatch boundary.
- profiling tùy capability: execution report và timestamp query primitives.

## Những gì cố ý không có trong core

- ECS, scene, animation, material, timeline hoặc editor;
- decoder/encoder asset/media;
- color management, tone mapping, alpha policy hoặc canonical export;
- UI fallback, event loop và host scheduling policy.

Host phải tạo `wgpu` resource, shader, pipeline, bind group và input bytes rồi
đăng ký bằng descriptor tương ứng. Core chỉ thực thi contract đó và trả typed
error/submission/raw readback.

## Evidence

Test library, integration test, benchmark và parity report nằm trong
[`tests/`](tests/). Các report phải ghi rõ scope Desktop/Web, input bytes,
graph fingerprint và giới hạn của phép đo; không dùng preview surface làm
source of truth.
