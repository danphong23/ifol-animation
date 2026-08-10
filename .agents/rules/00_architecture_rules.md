# Quy Tắc Kiến Trúc Mã Nguồn (Architecture & Codebase Rules)

Tài liệu này định nghĩa các ràng buộc sống còn về cách tổ chức mã nguồn trong Rust Workspace. Bất kỳ AI Agent nào khi tạo Crate mới hoặc viết mã đều **BẮT BUỘC** tuân thủ.

## 1. Cấu Trúc Thư Mục Tiêu Chuẩn

```text
crates/             # Các thư viện lõi, cấm gọi lên lớp vỏ (apps)
  ├── mcp-core      # Cổng giao tiếp AI Agent
  ├── ifol-math     # Toán học (Matrix, nội suy)
  ├── ifol-gpu      # GPU Engine (wgpu)
  ├── ifol-ecs      # Business Logic, Systems, Translate ra RenderGraph
  ├── ifol-asset    # Quản lý VFS, Bundle .ifol, FFmpeg
  └── ifol-app-core # Nắm giữ Singleton State và Command Bus

plugins/            # Gói mở rộng
  ├── plugin-nodes  # Component/System nghiệp vụ
  └── plugin-shaders# Mã nguồn WGSL

apps/               # Lớp vỏ hiển thị cao nhất
  ├── studio-desktop # Tauri (Native Surface)
  ├── studio-web     # WebAssembly (WebGPU)
  └── mcp-server-cli # Bản Headless
```

## 2. Quy Tắc Phụ Thuộc (Dependency Constraints)

Tuyệt đối cấm vi phạm luồng dữ liệu 1 chiều (One-way Data Flow) từ dưới lên:

1.  **Luật Cấm GPU:** `ifol-gpu` là tầng thấp nhất (chỉ sau `ifol-math`). Nó tuyệt đối **KHÔNG ĐƯỢC PHÉP** import `ifol-ecs`. GPU Engine không được biết khái niệm `Entity`, `Component` là gì. Nó chỉ nhận `RenderGraph`.
2.  **Luật Cấm Vòng (Circular Dependency):** `ifol-ecs` được gọi `ifol-gpu`. Không được có chiều ngược lại.
3.  **Luật Lớp Vỏ:** Các project trong thư mục `apps/` là trùm cuối. Nó gọi tất cả các `crates/` để ghép thành app. Không một crate lõi nào được phép gọi ngược (import) từ `apps/`.
4.  **Luật Sứ Giả:** Svelte UI (trong `apps/studio-*`) không được chọc thẳng vào `ifol-ecs`. Mọi tương tác UI phải bắn Command JSON xuống `ifol-app-core` (Command Bus), từ đó `ifol-app-core` mới gọi `ifol-ecs` để xử lý.
