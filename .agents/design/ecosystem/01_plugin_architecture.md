# Kiến Trúc Mở Rộng & Addon (Plugin Architecture)

Một phần mềm vĩ đại không bao giờ giới hạn tính năng ở phần lõi. Cấu trúc `ifol-animation` được thiết kế để Cộng Đồng hoặc các Dev bên thứ 3 có thể viết thêm tính năng mà không cần đụng vào mã nguồn Core Rust.

---

## 1. Mở Rộng GPU Engine (Custom Shaders)
GPU Engine hoàn toàn "mù" về cách vẽ. Vì vậy, để thêm một hiệu ứng đồ họa mới (Ví dụ: Glitch Effect, CRT Monitor Blur):
1.  Người dùng/Dev chỉ cần viết 1 file text `.wgsl` (WebGPU Shading Language) chứa thuật toán.
2.  Gửi file text đó vào hàm `register_pipeline("glitch_effect", wgsl_code)` của Engine lúc khởi tạo.
3.  ECS bây giờ có thể gắn `PipelineID = "glitch_effect"` vào một Draw Command.
👉 Hiệu ứng mới đã hoạt động mà không cần biên dịch lại phần mềm!

## 2. Mở Rộng Hệ Thống Node / Component (Custom Nodes)
*   Trong tương lai, giao diện Node Graph hoặc Timeline sẽ chứa rất nhiều loại Node (Khối hiệu ứng, Khối di chuyển).
*   Các Node này thực chất là các Component và System được định nghĩa động (dạng Scripting như Lua, Rhai, hoặc Javascript/WASM con).
*   Giao diện Svelte có thể tự do mở rộng (Plug & Play) các tab chức năng mới dưới dạng các Addon Component độc lập.

*(Tài liệu này sẽ được đi sâu hơn khi chúng ta thiết kế tính năng Plugin cụ thể).*
