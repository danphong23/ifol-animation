# Cấu Trúc Mã Nguồn & Bản Đồ Kiến Trúc (Architecture Overview)

Tài liệu này là **Bản đồ chỉ đường (Entry Point)** cho toàn bộ dự án `ifol-animation`. Nó định nghĩa quy hoạch thư mục mã nguồn (Codebase) của dự án và cách các module phụ thuộc vào nhau.

---

## 1. Bản Đồ Tài Liệu Thiết Kế (Design Map)

Dự án được chi phối bởi 10 tài liệu kiến trúc, chia thành 3 lớp từ thấp lên cao. Bạn phải tuân thủ nghiêm ngặt ranh giới của các lớp này:

*   **Lớp 1: Core Engine (Lõi Hệ Thống)**
    *   `01_ecs_manifest.md`: Khung xương ECS thuần túy.
    *   `gpu_engine/01_render_graph_and_command.md`: Cấu trúc mảng RenderNode và Hệ thống Handle.
    *   `gpu_engine/02_pipeline_and_shader.md`: Bản chất Shader Code, Shader Graph và quyền năng tạo WGSL của AI.
    *   `gpu_engine/03_memory_and_performance.md`: Ring Buffer (Zero-allocation), LRU Cache và Fast Texture Update.
    *   `gpu_engine/04_ecs_to_gpu_bridge.md`: Ranh giới trách nhiệm (Xử lý Video, Dịch Material Component ra Node).
    *   `04_render_execution_trace.md`: Câu chuyện mô phỏng luồng chạy 1 frame.
    *   `05_resource_lifecycle_and_ui_integration.md`: Khởi tạo Singleton, VRAM Cache, và Output ra màn hình.
*   **Lớp 2: Application Shell (Lớp Vỏ Ứng Dụng)**
    *   `01_platform_strategy.md`: Đa nền tảng (App, Web, Mobile).
    *   `02_ui_and_mcp_command_bus.md`: Cổng giao tiếp Command Bus, Live Sync giữa UI và AI.
    *   `03_project_and_asset_management.md`: Đóng gói Bundle `.ifol`, Đường dẫn tương đối, VFS.
*   **Lớp 3: Ecosystem (Hệ Sinh Thái Mở Rộng)**
    *   `01_plugin_architecture.md`: Cơ chế cắm Shader và Node từ bên ngoài.
    *   `02_versioning_and_migration.md`: Nâng cấp tương thích ngược file Project.

*(Tất cả các tài liệu trên đều bọc lót cho nhau, không có tài liệu nào thừa thãi, chúng phối hợp tạo thành một kiến trúc đóng gói hoàn hảo).*

---

## 2. Quy Hoạch Thư Mục Mã Nguồn (Rust Workspace)

Để hiện thực hóa bản đồ lý thuyết trên, mã nguồn vật lý (Thư mục dự án) sẽ được chia thành một **Rust Workspace** gồm nhiều Crate (Thư viện) độc lập.

```text
c:\Users\abc\.AI\Code\ifol-animation\
├── Cargo.toml                  # Quản lý toàn bộ Workspace
├── .agents/                    # [System] Chứa Rules và tài liệu Design
│
├── crates/                     # [Core Libraries] Các thư viện lõi không lệ thuộc UI
│   ├── mcp-core/               # Xử lý kết nối Model Context Protocol (AI Agent)
│   ├── ifol-math/              # Toán học ma trận, nội suy animation, vector
│   ├── ifol-gpu/               # GPU Engine thuần túy (wgpu, Render Graph, Pipeline)
│   ├── ifol-ecs/               # Lõi ECS, Translation Pipeline, DrawCache
│   ├── ifol-asset/             # Trình quản lý VFS, Bundle .ifol, Gọi FFmpeg
│   └── ifol-app-core/          # Nắm giữ Singleton State, Command Bus API (Cầu nối)
│
├── plugins/                    # [Extensions] Các gói mở rộng bọc ngoài lõi
│   ├── plugin-nodes/           # Các Component nghiệp vụ (Camera, Shape, Image, Comp)
│   └── plugin-shaders/         # Mã nguồn WGSL cho các hiệu ứng đồ họa
│
└── apps/                       # [Application Shell] Lớp vỏ hiển thị
    ├── studio-desktop/         # Tauri App (Chứa Svelte UI, gọi ifol-app-core, render Surface)
    ├── studio-web/             # WebAssembly App (Svelte UI, WebGPU, Local FFmpeg)
    └── mcp-server-cli/         # Bản Headless (Chỉ có MCP và Core, không có giao diện)
```

## 3. Quy Tắc Phụ Thuộc (Dependency Rules) - Cấm Vi Phạm

Để tránh tình trạng Code rác (Spaghetti code) và vòng lặp phụ thuộc (Circular Dependency), các Crate phải tuân thủ chiều mũi tên phụ thuộc từ trên xuống dưới:

1.  `ifol-gpu` **KHÔNG BAO GIỜ** được phép `import ifol-ecs`. GPU Engine không được biết khái niệm Entity là gì. Nó chỉ nhận `RenderGraph`.
2.  `ifol-ecs` **ĐƯỢC PHÉP** gọi `ifol-gpu` để định nghĩa cấu trúc dữ liệu gửi đi (DrawCommand).
3.  `plugins` **ĐƯỢC PHÉP** gọi `ifol-ecs` để đăng ký các Component mới.
4.  `apps/*` (Lớp vỏ) nằm ở tầng cao nhất. Nó gọi tất cả các Crate bên dưới để lắp ráp lại thành phần mềm chạy được. Không một Crate lõi nào được phép gọi ngược lên `apps`.
5.  `ifol-app-core` đóng vai trò là "Sứ Giả". Giao diện Svelte (trong `apps/`) sẽ ném lệnh (JSON) xuống `ifol-app-core`. Sứ giả này sẽ dịch lệnh đó ra và chỉ đạo ECS chạy.
