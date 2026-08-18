# Public usage guide

Đây là tài liệu dành cho người dùng bên ngoài `ifol-gpu`. Đọc theo thứ tự sau;
không cần đọc design internals trước.

Nếu tải crate độc lập, đọc thêm [crate README](../../README.md) trước để biết
trạng thái version, boundary và cách tích hợp dependency.

## Learning path

1. [Bắt đầu nhanh](87-guide-getting-started.md) — khởi tạo engine và hiểu ba
   tầng: host resource, graph, executor.
2. [Đăng ký resource](91-guide-resource-registration.md) — texture, buffer,
   pipeline, bind group và mesh descriptor.
3. [Pipeline và shader](88-guide-pipeline-and-shader.md) — phần host phải tự
   tạo và metadata core cần nhận.
4. [Xây dựng graph](89-guide-building-a-graph.md) — draw, compute, copy,
   dependency, usage và flatten.
5. [Execute, readback và lifecycle](92-guide-execution-and-lifecycle.md) —
   validation, submit, surface, readback và lifetime.
6. [Extension custom](90-guide-public-api-and-extensions.md) — mở rộng graph
   mà không đưa semantic engine vào core.
7. [API map và versioning](93-guide-api-map-and-versioning.md) — import path,
   ownership, error contract và mức ổn định của public API.
8. [Tra cứu public API](94-guide-public-api-reference.md) — method và type
   contract hiện hành.
9. [Mẫu tích hợp](95-guide-integration-recipes.md) — host resource, graph,
   extension và raw readback.

## Quy tắc quan trọng

- Core không tạo shader, material, texture data, video, animation hoặc editor.
- Host tạo `wgpu` resource, đăng ký bằng descriptor rồi graph chỉ giữ handle.
- Luôn dùng checked API và xử lý `Result`.
- Graph không sở hữu GPU resource; lifetime phải gắn với submission/frame policy
  của host.

## Tài liệu nâng cao

Khi cần hiểu implementation hoặc debug invariant, quay về [docs index](../README.md)
và chọn nhóm design tương ứng. Các nhóm đó không phải tutorial sử dụng.
