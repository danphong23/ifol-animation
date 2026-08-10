# Chiến Lược Đa Nền Tảng (Platform Strategy)

Tài liệu này định nghĩa kiến trúc lõi để phần mềm `ifol-animation` có thể chạy trên Desktop, Trình duyệt Web, và Thiết bị Di động mà **chỉ sử dụng một Codebase duy nhất**.

---

## 1. Triết Lý Kiến Trúc (Write Once, Run Everywhere)
Trái tim của phần mềm là lõi ECS và GPU Engine được viết bằng **Rust**. Thay vì phải viết lại logic cho từng hệ điều hành, chúng ta tận dụng khả năng biên dịch chéo (Cross-compilation) cực mạnh của Rust:
*   Biên dịch ra mã máy Native (`.exe`, `.app`, `.apk`).
*   Biên dịch ra WebAssembly (`.wasm`) để chạy trong trình duyệt.

## 2. Chiến Lược Cho Từng Nền Tảng

### 2.1. Desktop App (Windows, macOS, Linux)
*   **Công nghệ Vỏ (App Shell):** Sử dụng **Tauri**. Tauri sử dụng trình duyệt mặc định của HĐH (Edge/WebKit) để hiển thị giao diện Svelte, giúp dung lượng app chỉ còn khoảng vài MB (nhẹ hơn Electron 10 lần).
*   **Giao tiếp:** Svelte UI gọi lõi Rust thông qua Tauri IPC (Inter-Process Communication).
*   **Render:** GPU Engine sẽ xin HĐH một cái cửa sổ gốc (Native OS Window Handle) và vẽ trực tiếp bằng đồ họa phần cứng (DirectX 12/Metal/Vulkan). Tốc độ tối đa tuyệt đối.

### 2.2. Web App (Client-side 100%)
*   **Công nghệ Vỏ:** Trình duyệt Web thông thường (Chrome, Edge). Giao diện Svelte được đóng gói thành file HTML/JS tĩnh.
*   **Lõi Rust:** Được biên dịch thành file `engine.wasm`.
*   **Render (WebGPU):** Engine WASM chọc thẳng vào card màn hình của người dùng thông qua API WebGPU. Trình duyệt không có cửa sổ hệ điều hành, nên GPU Engine sẽ vẽ kết quả ra một thẻ `<canvas>`.
*   **Không tốn tiền máy chủ (Zero Server Cost):** Toàn bộ việc render và xuất video (sử dụng FFmpeg WASM) đều chạy cục bộ trên máy người dùng (Client-side). Máy chủ duy nhất ta cần là máy chủ lưu trữ file HTML tĩnh (như Github Pages hoặc Vercel).

### 2.3. Mobile App (iOS, Android)
*   **Công nghệ Vỏ:** Tauri Mobile hoặc các framework native.
*   **Tích hợp lõi:** Biên dịch lõi Rust thành thư viện động/tĩnh C-ABI (C Application Binary Interface). 
*   **Mở rộng:** App Swift (iOS) hoặc Kotlin (Android) gọi trực tiếp các hàm C-ABI này để giao tiếp với ECS. GPU Engine dùng Metal (iOS) hoặc Vulkan (Android) để vẽ thẳng lên màn hình điện thoại. Kiến trúc hoàn toàn khả thi và nhất quán!
