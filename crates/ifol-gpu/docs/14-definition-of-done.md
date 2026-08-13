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
