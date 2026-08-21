# TC06 — rollback khi lỗi và shutdown

Trạng thái: PASS

Đầu vào: đăng ký phase tạo cycle, sau đó shutdown sau khi reconfigure thất bại.

Kỳ vọng: staging thất bại nhưng runtime hiện tại vẫn `Ready`; shutdown thành
công chuyển sang `ShuttingDown`.
