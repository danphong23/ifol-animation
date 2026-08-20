# Chiến Lược Đa Nền Tảng Và Capability Adapters

Tài liệu này định nghĩa cách tái sử dụng cùng kernel/feature contracts trên
Desktop, Web, Mobile và CLI. Mục tiêu là dùng chung business core, không giả định
mọi platform có implementation, codec, filesystem hay GPU capability giống nhau.

---

## 1. Triết Lý Kiến Trúc

`ifol-engine`, `ifol-ecs`, schema và feature logic thuần Rust được dùng chung.
Subsystem/platform service được thay bằng adapter theo capability:
*   Biên dịch ra mã máy Native (`.exe`, `.app`, `.apk`).
*   Biên dịch ra WebAssembly (`.wasm`) để chạy trong trình duyệt.

## 2. Chiến Lược Cho Từng Nền Tảng

### 2.1. Desktop App (Windows, macOS, Linux)
*   **Công nghệ Vỏ (App Shell):** Sử dụng **Tauri**. Tauri sử dụng trình duyệt mặc định của HĐH (Edge/WebKit) để hiển thị giao diện Svelte, giúp dung lượng app chỉ còn khoảng vài MB (nhẹ hơn Electron 10 lần).
*   **Giao tiếp:** Svelte UI gọi lõi Rust thông qua Tauri IPC (Inter-Process Communication).
*   **Render:** platform adapter sở hữu window/surface lifecycle và truyền boundary
    hợp lệ cho Render Core/GpuService. Backend thực tế phụ thuộc adapter/capability.

### 2.2. Web App
*   **Công nghệ Vỏ:** Trình duyệt Web thông thường (Chrome, Edge). Giao diện Svelte được đóng gói thành file HTML/JS tĩnh.
*   **Lõi Rust:** Được biên dịch thành file `engine.wasm`.
*   **Render (WebGPU):** Engine WASM chọc thẳng vào card màn hình của người dùng thông qua API WebGPU. Trình duyệt không có cửa sổ hệ điều hành, nên GPU Engine sẽ vẽ kết quả ra một thẻ `<canvas>`.
* Client-side render là profile mục tiêu. Decode/encode có thể dùng WebCodecs,
  WASM codec hoặc fallback khác; không bắt buộc FFmpeg WASM và không cam kết mọi
  export chạy client-side trước khi có capability/runtime evidence.

### 2.3. Mobile App (iOS, Android)
*   **Công nghệ Vỏ:** Tauri Mobile hoặc các framework native.
*   **Tích hợp lõi:** Có thể dùng Tauri Mobile, Rust bridge hoặc C-ABI tùy kết quả
    spike; C-ABI không phải contract bắt buộc từ đầu.
*   **Render/media:** dùng adapter phù hợp Metal/Vulkan/codec platform nếu được
    hỗ trợ. Mobile chỉ được đánh dấu supported sau compile + runtime evidence.

---

## 3. Platform Service Contracts

Engine/feature chỉ phụ thuộc interface cho VFS, task execution, clock, surface,
clipboard/input và codec capability. Adapter được khởi tạo bên ngoài rồi truyền
vào `EngineRuntime`; core không gọi ngược lên `apps/*`.

CLI là platform profile không UI và phải là đường kiểm chứng đầu tiên cho open,
mutate, tick, render và export. Desktop/Web/Mobile gọi trực tiếp headless engine
API; chúng không phụ thuộc binary `ifol-cli`.

Mọi tuyên bố parity phải phân biệt:

- compile support;
- runtime support;
- visual/correctness evidence;
- performance evidence;
- fallback/unsupported capability.
