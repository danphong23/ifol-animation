# Tài liệu `ifol-gpu`

## Người dùng bên ngoài

Bắt đầu tại [crate README](../README.md) để biết trạng thái package, public
module map và boundary khi tích hợp như thư viện.

Bắt đầu tại [public usage guide](60-guides/README.md). Đây là lộ trình chính
cho engine, game, animation, video pipeline và các ứng dụng GPU bên ngoài.

## Contributor: đọc trước khi sửa core

Đọc [ý định kiến trúc và workflow](00-foundation/16-current-intent-and-refactor-workflow.md),
sau đó đọc [kế hoạch tách module từng bước](00-foundation/17-incremental-module-splitting-plan.md).

Đối với asset, parity và file output, đọc thêm [canonical render và media output
contract](00-foundation/18-canonical-render-and-media-output-contract.md) và
[chính sách kiểm thử parity](00-foundation/19-cross-platform-parity-testing-policy.md).
Hai tài liệu này là boundary chính thức: decoder, color/alpha policy và
encoder thuộc tầng ngoài; `ifol-gpu` chỉ nhận contract, execute và raw readback.

Điểm bàn giao hiện tại nằm ở
[baseline và handoff](70-status/88-current-handoff-baseline.md).
Kết quả regression và Desktop/Web canonical parity gần nhất nằm ở
[báo cáo nâng cấp và parity](70-status/89-upgrade-regression-and-parity.md).
Ranh giới chứng nhận, baseline sạch và trạng thái TC đang chờ xử lý nằm ở
[validation boundary và clean baseline](70-status/90-validation-boundary-and-clean-baseline.md).

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

Tài liệu kiến trúc nền tảng gồm [architecture contracts](00-foundation/20-architecture-contracts.md).
Tài liệu sử dụng public gồm [API reference](60-guides/94-guide-public-api-reference.md)
và [integration recipes](60-guides/95-guide-integration-recipes.md).

`docs/` là design source chính thức của crate; `.agents/design` chỉ là lịch sử
hoặc tài liệu điều phối cấp workspace.
