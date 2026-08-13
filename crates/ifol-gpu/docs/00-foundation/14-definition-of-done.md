# IFOL GPU: Definition of Done

Core release không yêu cầu shader reflection, video/editor, golden-image
framework hoặc benchmark dashboard trong runtime crate. Những mục đó được
đánh giá ở tool/engine/test harness bên ngoài; core chỉ yêu cầu contract và
evidence tương ứng với boundary trong [task map](15-core-boundaries-and-task-map.md).

## Done cho một task

- code compile với feature/config liên quan;
- test behavior và edge case pass;
- không thêm panic trên public invalid path;
- không thêm hard-code không có design reason;
- docs và migration note được cập nhật;
- regression gate không giảm.

## Done cho core release candidate

Core chỉ được gọi là release candidate khi resource lifetime, graph dependency,
frame memory, surface format, cache invalidation và structured errors đều có
evidence; baseline runtime chạy trên các platform mục tiêu khả dụng; test plan
có owner/status; và không còn tài liệu tuyên bố capability thiếu evidence.

## Audit hiện tại

| Gate | Trạng thái hiện tại |
|---|---|
| Resource lifetime/stale handle | Đạt nền tảng; FrameContext và registry deferred ownership đã có |
| Graph dependency/pass execution | Đạt nền tảng; flat plan và hazard validation đã có |
| Frame memory submission-safe | Đạt nền tảng; transient pools, tracker, frame seal/reset đã có |
| Surface format không hard-code | Đạt nền tảng; resize/reconfigure typed, present thuộc host |
| Cache invalidation | Đạt nền tảng; resource/sample/context key đã có |
| Structured errors | Đạt; validation trước submit |
| Dynamic offsets/layout metadata | Đạt nền tảng; reflection đầy đủ còn thiếu |
| Cross-platform runtime matrix | Chưa đạt; mới có host runtime, WASM/MSVC compile và backend probes |
| MSAA/resolve/indirect | Đạt nền tảng; capability matrix và fixture đa backend còn thiếu |
| Async readback/profiling | Đạt nền tảng; pass-level/đa frame còn thiếu |

Vì runtime matrix, reflection và một số capability gates chưa đủ evidence, core
chưa được gọi là release candidate. Xem [audit hiện tại](../70-status/80-current-audit.md)
để biết lệnh gate và phạm vi bằng chứng.
