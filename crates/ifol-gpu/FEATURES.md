# Trạng thái tính năng IFOL GPU

Đây là bảng trạng thái, không phải tài liệu đặc tả kiến trúc. Thiết kế chính thức nằm trong [`docs/README.md`](docs/README.md).

## Chú thích trạng thái

- **Prototype đã implement**: đã có và được chạy trong crate hiện tại;
- **Một phần**: đã có nhưng chưa an toàn production hoặc chưa hoàn chỉnh;
- **Đã lên kế hoạch**: mục tiêu thiết kế, chưa implement;
- **Ngoài phạm vi**: thuộc crate cấp cao hơn.

## Backend và device

| Khả năng | Trạng thái | Ghi chú |
|---|---|---|
| Khởi tạo `wgpu` device/queue | Prototype đã implement | Đã có headless initialization. |
| Adapter capability snapshot | Một phần | Cần feature negotiation và capability model đầy đủ hơn. |
| Chọn backend rõ ràng | Một phần | Builder lưu cấu hình nhưng instance creation phải thực sự dùng nó. |
| Cấu hình required feature/limit | Một phần | Field nội bộ có nhưng public builder API chưa đủ. |
| Surface/presentation integration | Một phần | Cần thiết kế lại lifecycle và format handling. |
| Ma trận Windows/macOS/Linux/Web/mobile | Đã lên kế hoạch | Có nền tảng `wgpu`, chưa có platform validation đầy đủ. |

## Resource và memory

| Khả năng | Trạng thái | Ghi chú |
|---|---|---|
| Texture/pipeline/mesh/bind-group handle | Prototype đã implement | Handle số đơn giản; generational handle là kế hoạch. |
| Resource registry | Một phần | Raw map public phù hợp thử nghiệm, chưa phải API cuối. |
| Uniform ring buffer | Một phần | Alignment hoạt động; thiếu synchronization với GPU in-flight. |
| Texture pool/cache | Một phần | Chỉ là exact-match free-list, chưa phải LRU hay VRAM eviction. |
| Transient resource allocator | Đã lên kế hoạch | Cần graph lifetime analysis và submission tracking. |
| Structured resource error | Đã lên kế hoạch | API hiện còn string error và silent skip. |

## Graph và execution

| Khả năng | Trạng thái | Ghi chú |
|---|---|---|
| Ordered render graph | Prototype đã implement | Graph hiện là ordered node list có nesting. |
| Indexed/procedural draw | Prototype đã implement | Map tới draw call cơ bản của `wgpu`. |
| Offscreen target | Prototype đã implement | Cần validate descriptor thật sự. |
| Depth attachment | Prototype đã implement | MSAA/stencil/resolve model rộng hơn là kế hoạch. |
| Render bundle cache | Một phần | Context key và dynamic-data invalidation chưa đầy đủ. |
| Dependency-aware graph compiler | Đã lên kế hoạch | Cần cho multi-pass, compute và transient resource. |
| Compute pass | Đã lên kế hoạch | Phải dùng chung resource/dependency model với render pass. |
| Copy/resolve/mipmap pass | Đã lên kế hoạch | Cần cho GPU work composition đầy đủ. |
| Indirect draw/dispatch | Đã lên kế hoạch | Phụ thuộc capability. |
| Nhiều submission trong một frame | Policy đã lên kế hoạch | API không được bắt buộc single submission. |

## Validation và testing

| Khả năng | Trạng thái | Ghi chú |
|---|---|---|
| Unit test cơ bản | Prototype đã implement | Coverage hiện còn nhỏ. |
| Headless/device integration test | Một phần | Phụ thuộc môi trường. |
| Visual snapshot example | Prototype đã implement | Examples chưa phải test suite authoritative sạch. |
| Cross-backend test matrix | Đã lên kế hoạch | Cần cho portability claim. |
| Structured validation diagnostic | Đã lên kế hoạch | Phải thay silent missing-resource skip. |
| Tách benchmark theo từng giai đoạn | Một phần | Benchmark hiện trộn encoding, submission và GPU wait. |

## Ngoài phạm vi

- ECS và scene management;
- animation và timeline system;
- asset import, video decode và project file;
- UI/editor/MCP command bus;
- gameplay, physics, audio và domain-specific fallback.

## Milestone core tiếp theo

1. Chốt public contract cho resource/handle/error.
2. Sửa backend selection và surface format handling.
3. Làm frame memory và resource reuse an toàn theo submission.
4. Tách logical graph khỏi compiled/cache state.
5. Thiết kế dependency-aware render/compute/copy graph.
6. Sửa toàn bộ example và thiết lập integration-test baseline đáng tin cậy.
