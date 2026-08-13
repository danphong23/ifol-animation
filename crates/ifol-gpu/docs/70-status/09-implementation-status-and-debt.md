# IFOL GPU: trạng thái implementation và design debt

Tài liệu này phân biệt behavior đã có test với phần còn là policy hoặc kế
hoạch. Snapshot chi tiết mới nhất nằm ở [audit hiện tại](../70-status/80-current-audit.md).

## Đã implement và có test gate

- khởi tạo device/queue, capability snapshot và builder backend/fallback policy;
- generational handle, registry descriptor/version, owned texture và deferred
  destruction;
- graph flatten, dependency explicit/automatic hazard theo subresource;
- render, compute, copy, indirect, MSAA/resolve, depth/stencil và surface
  resize/reconfigure;
- `SubmissionTracker`, transient pools và `FrameContext` submission-safe;
- async readback, execution report, timestamp query pool và tracked profiling;
- dynamic bind-group limit, dynamic offset metadata và pipeline-layout signature;
- bundle cache invalidation theo resource, format, sample count và context key.

## Đã có nhưng còn giới hạn

- view-only texture không giữ ownership; path cần copy/resolve/lifetime phải
  dùng owned texture descriptor API;
- layout signature là metadata do host cấp, chưa phải shader reflection;
- bundle fast path và segmented path có policy khác nhau; correctness ưu tiên
  validation trước tối ưu;
- present/acquire/retry, map readback và completion notification thuộc host;
- timestamp boundary hiện đo toàn graph, chưa tự chèn timestamp từng pass.

## Chưa hoàn thiện

- runtime cross-platform matrix cho Metal, Linux, browser WebGPU, Android, iOS;
- capability/format matrix theo adapter và visual parity đa backend;
- reflection kiểm tra binding type, visibility và min binding size;
- pass-level profiling, queue nhiều frame và worker scheduling.

## Design debt còn lại

- một số examples cũ còn warning hoặc `unwrap` phục vụ assertion;
- readback/save boundary hiện chỉ còn checked typed-error API; host chịu trách
  nhiệm chuyển lỗi thành thông báo giao diện nếu cần;
- cần tiếp tục audit các nhánh encoder sau validation khi command model mở rộng.

Không rewrite toàn bộ crate. Graph model, handle, resource registry, memory
primitive và visual fixture có test được giữ lại; chỉ thay boundary khi invariant
mới có test và tài liệu tương ứng.
