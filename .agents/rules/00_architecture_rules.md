# Quy Tắc Kiến Trúc Mã Nguồn (Architecture & Codebase Rules)

Tài liệu này định nghĩa các ràng buộc sống còn về cách tổ chức mã nguồn trong Rust Workspace. Bất kỳ AI Agent nào khi tạo Crate mới hoặc viết mã đều **BẮT BUỘC** tuân thủ.

## 1. Cấu Trúc Thư Mục Mục Tiêu

```text
crates/             # Kernel, headless host và các subsystem mù
  ├── ifol-ecs      # Pure ECS runtime
  ├── ifol-engine   # Headless composition, project/package/session/step
  └── ifol-gpu      # Agnostic GPU subsystem

features/           # Gói tính năng đăng ký vào engine/ECS/subsystems
  ├── feature-render-core
  └── feature-shape

apps/               # Adapters; không chứa business logic
  ├── ifol-cli
  ├── studio-desktop
  ├── studio-web
  └── mcp-server
```

Các crate/feature tương lai chỉ tạo khi bắt đầu triển khai use case thật. Danh
sách mục tiêu trong design không phải yêu cầu tạo placeholder ngay lập tức.

## 2. Quy Tắc Phụ Thuộc (Dependency Constraints)

Tuyệt đối cấm vi phạm luồng dữ liệu 1 chiều (One-way Data Flow) từ dưới lên:

1. **Luật Hai Kernel Mù:** `ifol-gpu` và `ifol-ecs` không import hoặc biết
   semantic của nhau. `feature-render-core` là consumer của cả hai và là cầu nối.
2. **Luật Subsystem Mù:** Asset/decode/encode/font/3D/GPU chỉ nhận descriptor
   chuẩn hóa và trả artifact/report. Chúng không import ECS hoặc feature domain.
3. **Luật Feature Một Chiều:** Feature được phụ thuộc kernel, subsystem và
   foundation feature thấp hơn; chiều ngược lại bị cấm. Dependency giữa feature
   phải được khai báo bằng stable `FeatureId` và không tạo cycle.
4. **Luật Engine Host:** `ifol-engine` là composition root, khởi tạo ECS,
   services và feature packages. ECS không tự discover project/plugin/service.
5. **Luật Loop:** `apps/*`, CLI hoặc worker giữ platform/job loop;
   `ifol-engine` chỉ cung cấp `step()` hữu hạn và không tự sleep/retry/run-forever.
6. **Luật Adapter:** `apps/*` chỉ gọi API của `ifol-engine`; không chứa business
   logic và không mutate ECS World trực tiếp.
7. **Luật Command:** External mutation đi qua typed command contract do package
   đăng ký hoặc typed step input; engine không có enum command nghiệp vụ trung tâm.
   JSON chỉ là wire format ở transport boundary.
8. **Luật CLI First:** Feature phải có đường chạy/test headless trước khi nối UI.
9. **Luật Service Instance:** Service "singleton" là instance thuộc một
   `EngineRuntime`, được truyền bằng interface/handle; cấm global mutable static.
10. **Luật World Hợp Nhất:** Dữ liệu/service toàn World là component trên
   `WORLD_ENTITY`; cấm tạo resource registry/storage/change tracker song song.
   Query bình thường được phép khớp `WORLD_ENTITY`.
11. **Luật Namespace:** Engine chỉ sở hữu manifest/lock/scenes và namespace
   container generic; package sở hữu nội dung `runtime/<package-id>` và migration.
