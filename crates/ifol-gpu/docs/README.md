# Tài liệu `ifol-gpu`

## Bạn chỉ muốn sử dụng thư viện?

Bắt đầu tại [Public usage guide](60-guides/README.md). Đây là learning path
chính để xây engine, tool, game, video pipeline hoặc ứng dụng GPU bên ngoài:

1. [Bắt đầu nhanh](60-guides/87-guide-getting-started.md)
2. [Đăng ký resource](60-guides/91-guide-resource-registration.md)
3. [Pipeline và shader](60-guides/88-guide-pipeline-and-shader.md)
4. [Xây dựng graph](60-guides/89-guide-building-a-graph.md)
5. [Execute, readback và lifecycle](60-guides/92-guide-execution-and-lifecycle.md)
6. [Extension custom](60-guides/90-guide-public-api-and-extensions.md)

Không cần đọc các thư mục design bên dưới để bắt đầu dùng API.

## Bạn đang phát triển chính `ifol-gpu`?

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
