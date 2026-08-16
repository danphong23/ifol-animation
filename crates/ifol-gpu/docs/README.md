# Tài liệu `ifol-gpu`

## 🏛️ Bộ Tài Liệu Chuẩn Công Nghiệp & Tích Hợp (Core Architecture & API Guides)

Dành cho các Crate bên ngoài (`ifol-app-core`, `ifol-ecs`, `ifol-media`) và các nhà phát triển tích hợp:

1. 📜 [**00-architecture-contracts.md**](00-architecture-contracts.md): Hợp đồng kiến trúc & Ranh giới trách nhiệm ("Blind Agnostic GPU Engine").
2. 📖 [**01-public-api-reference.md**](01-public-api-reference.md): Bảng tra cứu toàn bộ Public API, Structs, Enums, Handles và Graph Nodes.
3. 🍳 [**02-integration-guide-and-recipes.md**](02-integration-guide-and-recipes.md): Hướng dẫn tích hợp thực tế (Desktop/Web Init, Video NV12, Native Plugin Extensions, ECS Translation).

## 🚀 Public Usage Guide & Học Nhanh
Bắt đầu tại [Public usage guide](60-guides/README.md):

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
