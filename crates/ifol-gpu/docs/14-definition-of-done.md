# IFOL GPU: Definition of Done

## Done cho một task

Task chỉ hoàn thành khi:

- code compile với feature/config liên quan;
- test mới hoặc test cập nhật đã pass;
- test edge case liên quan đã pass;
- không có `unwrap`/panic mới trên public invalid path;
- không tạo hard-code mới nếu không có lý do được ghi lại;
- docs liên quan được cập nhật;
- API change có migration note;
- không làm hỏng regression test.

## Done cho một phase

Phase chỉ hoàn thành khi:

- tất cả task trong phase pass;
- test gate của phase pass;
- full regression pass;
- visual fixture không regression ngoài tolerance;
- implementation status phản ánh đúng thực tế;
- design decision quan trọng đã được ghi trong docs/ADR nếu cần;
- phase kế tiếp có input ổn định.

## Done cho core release candidate

`ifol-gpu` chỉ được coi là release candidate khi:

- resource lifetime và stale handle được kiểm soát;
- graph dependency và pass execution được validate;
- frame memory submission-safe;
- surface format không hard-code;
- cache invalidation deterministic;
- structured errors đầy đủ;
- baseline chạy trên các platform mục tiêu khả dụng;
- examples không còn là cách duy nhất để chứng minh behavior;
- toàn bộ test plan bắt buộc có owner/status;
- không còn tài liệu tuyên bố capability chưa có bằng chứng.

## Audit trạng thái hiện tại

| Gate | Trạng thái | Evidence/ghi chú |
|---|---|---|
| Resource lifetime, stale handle | Đạt một phần | generational handle, owned resource, transient texture pool và submission-safe ring đã có test; deferred destruction tổng quát còn thiếu |
| Graph dependency/pass execution | Đạt nền tảng | flat graph, explicit + automatic hazard edges, mip/layer subresource hazard metadata và render/compute/copy segmented execution đã có test |
| Frame memory submission-safe | Đạt một phần | ring reset gate, submission tracker và transient texture/buffer pool đã có; frame context/deferred destruction còn thiếu |
| Surface format không hard-code | Đạt | format lấy từ surface config; surface lost/present lifecycle còn thiếu |
| Cache invalidation | Đạt nền tảng | resource versions và bundle key đã có; multi-context cache còn thiếu |
| Structured errors | Đạt | public `execute`/`execute_with_surface` trả `Result`; encoder unchecked chỉ chạy sau validation |
| Cross-platform runtime matrix | Chưa đạt | chỉ có evidence trên môi trường hiện tại; chưa chạy đủ Windows/macOS/Linux/Web/Android/iOS |
| MSAA/resolve | Đạt một phần | color resolve, depth sample matching và stencil aspect cơ bản qua `OffscreenMsaa` đã có runtime test; subresource/capability matrix còn thiếu |
| Indirect draw/dispatch | Đạt nền tảng | command model, range/usage validation và encoder path đã có; end-to-end fixture đa backend còn thiếu |
| Async readback/profiling | Chưa đạt | readback sync/format-aware hiện có, contract async/profiling hook còn thiếu |

Vì các gate “Chưa đạt” và “Đạt một phần” còn tồn tại, core hiện chưa được gọi là
release candidate. Mốc hiện tại là một nền tảng design/implementation có test
regression mạnh trên host hiện tại.
