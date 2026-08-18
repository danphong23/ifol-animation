# Status và release evidence

Nhóm này ghi evidence của source/test hiện tại. Đây không phải user manual và
không thay thế API reference.

- [Implementation status và debt](09-implementation-status-and-debt.md)
- [Current audit](80-current-audit.md)
- [Typed readback errors](81-typed-readback-errors.md)
- [Core cleanup và roadmap](86-core-cleanup-and-roadmap.md)
- [Core baseline release audit](87-core-baseline-release-audit.md)
- [Current handoff baseline](88-current-handoff-baseline.md)
- [Upgrade regression và parity](89-upgrade-regression-and-parity.md)
- [Validation boundary và clean baseline](90-validation-boundary-and-clean-baseline.md)

Quy tắc đọc status:

- “Đã có test gate” chỉ nói behavior đã được kiểm thử trong scope ghi rõ;
- compile hoặc một backend probe không chứng minh runtime parity trên mọi nền tảng;
- sai khác decoder, canvas, driver hoặc encoder phải được phân loại ở tầng ngoài,
  không gán cho core nếu raw graph contract vẫn đúng;
- baseline chứng nhận hiện tại và các TC pending phải được đọc từ handoff mới nhất.
