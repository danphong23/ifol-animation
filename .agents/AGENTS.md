# 🤖 AI Agent Entry Point (ifol-animation)

Chào mừng AI Agent (Antigravity/Cursor/Claude). Mọi hành động sửa code, tạo file của bạn trong dự án này đều **BẮT BUỘC** phải tuân thủ các Rules và Workflows trong thư mục `.agents/`.

---

## 1. Bản Chất Dự Án `ifol-animation`

`ifol-animation` là phần mềm Motion Graphics Editor lai (Hybrid) hiệu năng cao:
*   **Mục tiêu cao nhất:** Tối giản hóa tối đa, tránh sự phức tạp dư thừa, xây dựng từng phần độc lập và kiểm thử chặt chẽ (Edge cases).
*   **Kiến trúc:** Pure ECS kết hợp GPU Render Engine mù quáng (Agnostic).
*   **Đa Nền Tảng:** Core Rust dùng chung cho Desktop (Tauri/wgpu) và Web (WASM/WebGPU). Mọi thay đổi dữ liệu (UI/MCP) đều thông qua Single Command Bus.

## 2. Quy Tắc Vàng (Vòng Ràng Buộc Hệ Thống)

Mọi quy tắc về kiến trúc và chất lượng mã nguồn đều được quy định nghiêm ngặt tại đây. Bạn **BẮT BUỘC** phải đọc trước khi tạo hoặc sửa Crate:
1. 👉 **[Xem Quy Tắc Cấu Trúc Mã Nguồn](file:///c:/Users/abc/.AI/Code/ifol-animation/.agents/rules/00_architecture_rules.md)** (Phụ thuộc vòng, phân chia Crate).
2. 👉 **[Xem Chiến Lược Kiểm Thử Bắt Buộc](file:///c:/Users/abc/.AI/Code/ifol-animation/.agents/rules/01_testing_strategy.md)** (Hướng dẫn tư duy Test-Driven và Edge cases).

### 2.0. Cấm Edit File Bằng Terminal (Công Cụ Sửa Đổi)
*   Tuyệt đối **KHÔNG** sử dụng các lệnh terminal như `echo`, `sed`, `awk` để tạo hoặc sửa file mã nguồn. 
*   Việc làm này sẽ khiến IDE (Antigravity/Cursor) không nhận diện được file nào đã thay đổi. Bắt buộc phải dùng các công cụ chuyên dụng (`write_to_file`, `replace_file_content`).

### 2.1. Chuẩn hóa Dự Án Đóng (Self-contained Project)
*   Dự án lưu dưới định dạng gói Bundle `.ifol` (zip chứa `project.json` và `/assets/`).
*   Mọi file asset (video, image) phải được tham chiếu bằng **đường dẫn tương đối** qua hệ thống VFS (Virtual File System).

### 2.2. Unified Command Bus (Sự Bình Đẳng UI & AI)
*   Tất cả thay đổi trạng thái của World (Svelte UI hay MCP) đều phải đi qua `CommandBus` của `ifol-app-core`.
*   Cấm ngặt việc giao diện UI truy cập trực tiếp và thay đổi (mutate) State của ECS. Mọi hành động đều là phát Event / gửi Command.

---

## 3. Lệnh Đọc Bắt Buộc (Mandatory Reading)
Để hiểu được triết lý sâu xa của dự án trước khi viết dù chỉ 1 dòng code, bạn **BẮT BUỘC PHẢI ĐỌC TOÀN BỘ 10 TÀI LIỆU THIẾT KẾ** nằm trong thư mục `.agents/design/`. 

Hãy bắt đầu bằng việc đọc tấm bản đồ dẫn đường này trước:
👉 **[.agents/design/00_architecture_overview_and_codebase.md](file:///c:/Users/abc/.AI/Code/ifol-animation/.agents/design/00_architecture_overview_and_codebase.md)**

Sau khi đọc file `00` ở trên, nó sẽ chỉ dẫn bạn đọc tiếp 10 tài liệu còn lại (được chia làm 3 lớp: Core Engine, Application Shell, Ecosystem). **Đừng bỏ sót bất kỳ file nào.** Chỉ khi bạn đã thẩm thấu 100% triết lý của 10 tài liệu này, bạn mới đủ tư cách để đề xuất hoặc sửa đổi mã nguồn.
