# IFOL GPU: Trạng thái implementation và design debt

Tài liệu này phân biệt behavior đã được kiểm chứng với phần mới chỉ là design
hoặc policy dự kiến.

## Đã implement và có test gate

- khởi tạo `wgpu` device/queue và capability snapshot;
- screen/offscreen target, indexed/procedural draw, depth attachment và clear;
- graph nesting, flatten, explicit dependency ordering và cycle validation;
- generational handle allocator và stale-handle detection;
- `ResourceRegistry` có insert/lookup/remove/version tracking, descriptor
  validation và private resource maps;
- owned texture resource với descriptor metadata;
- render, compute và copy node; buffer-to-buffer và texture-to-texture copy;
- graph không có render target và execution segmented giữ đúng thứ tự
  copy/compute/draw;
- validation typed cho missing resource, target size/usage, copy range, texture
  mip/range/format/ownership và buffer copy usage;
- readback theo texture format;
- bundle cache key có resource version;
- `SubmissionTracker`, ring allocation không implicit wrap và reset có gate
  completion;
- transient texture pool exact-match với descriptor đầy đủ, in-flight protection,
  duplicate-release detection và drain sau completion;
- examples, integration test và benchmark harness cơ bản.

## Đã implement nhưng còn giới hạn

- render bundle mới tối ưu fast path render thuần; segmented path encode draw
  trực tiếp để ưu tiên correctness;
- texture registry compatibility API view-only chưa có đủ descriptor/ownership;
  copy/resolve/lifetime phải dùng owned texture API;
- `TextureCache` vẫn tồn tại như type alias compatibility cho
  `TransientTexturePool`, không phải LRU hay VRAM eviction manager;
- lock surface poisoning được xử lý không panic, nhưng lifecycle surface lost,
  reconfigure và resize policy đa nền tảng còn cần hoàn thiện;
- bind-group state cache hiện giới hạn bốn slot.

## Chưa implement

- resource hazard model khai báo read/write usage tổng quát trong graph;
- MSAA resolve, depth/stencil aspect và subresource model đầy đủ;
- indirect draw/dispatch;
- capability tier và fallback policy theo từng backend/platform;
- cross-backend/device matrix tự động cho Windows, macOS, Web, Android và iOS;
- frame context hoàn chỉnh, transient buffer pool và deferred destruction tích
  hợp trực tiếp với GPU completion;
- async readback contract tổng quát và profiling/diagnostics hook đầy đủ;
- context-aware bundle cache cho nhiều viewport/device.

## Design debt hiện tại

- một số test/example còn `unwrap` để làm failure assertion; production path cần
  tiếp tục audit riêng;
- format/usage metadata của các resource đăng ký qua compatibility API chưa đầy
  đủ, nên validation mạnh yêu cầu API descriptor;
- execution compiler vẫn có silent-skip ở low-level `execute` legacy API; caller
  production nên dùng `execute_checked`;
- tài liệu cũ về copy/compute cần được cập nhật tiếp để phản ánh texture copy và
  ordered segments;
- warning/style debt còn lại trong example user-owned.

## Chính sách rewrite

Không rewrite mù toàn bộ crate. Các experiment, visual fixture, graph model và
API đã được test giữ lại làm nền; chỉ thay thế từng boundary khi invariant mới
đã có test và tài liệu tương ứng.
