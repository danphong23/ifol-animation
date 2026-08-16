# Tài liệu `ifol-gpu`

## Người dùng bên ngoài

Bắt đầu tại [public usage guide](60-guides/README.md). Đây là lộ trình chính
cho engine, game, animation, video pipeline và các ứng dụng GPU bên ngoài.

## Contributor: đọc trước khi sửa core

Đọc [ý định kiến trúc và workflow](00-foundation/16-current-intent-and-refactor-workflow.md),
sau đó đọc [kế hoạch tách module từng bước](00-foundation/17-incremental-module-splitting-plan.md).

Điểm bàn giao hiện tại nằm ở
[baseline và handoff](70-status/88-current-handoff-baseline.md).

Các tài liệu design, invariant, test, audit và migration được giữ lại nhưng
không phải user manual:

- [Foundation và scope](00-foundation/README.md)
- [Graph internals](10-graph/README.md)
- [Resource internals](20-resources/README.md)
- [Execution internals](30-execution/README.md)
- [Memory internals](40-memory/README.md)
- [Platform/backend](50-platform/README.md)
- [Status và release audit](70-status/87-core-baseline-release-audit.md)
- [Migration history](80-migration/README.md)

`docs/` là design source chính thức của crate; `.agents/design` chỉ là lịch sử
hoặc tài liệu điều phối cấp workspace.
