# 🤖 AI Agent Entry Point (ifol-animation)

Chào mừng AI Agent (Antigravity/Cursor/Claude). Mọi hành động sửa code, tạo file của bạn trong dự án này đều **BẮT BUỘC** phải tuân thủ các Rules và Workflows trong thư mục `.agents/`.

---

## 1. Bản Chất Dự Án `ifol-animation`

`ifol-animation` là phần mềm Motion Graphics Editor lai (Hybrid) hiệu năng cao:
*   **Mục tiêu cao nhất:** Tối giản hóa tối đa, tránh sự phức tạp dư thừa, xây dựng từng phần độc lập và kiểm thử chặt chẽ (Edge cases).
*   **Kiến trúc:** Runnable ladder gồm ECS kernel, các subsystem mù, headless
    Engine Host và Feature Packages. `ifol-ecs` không biết `ifol-gpu`; cầu nối
    render nằm trong `feature-render-core`.
*   **Đa Nền Tảng:** Core Rust dùng chung cho Desktop, Web, CLI và worker. Host
    giữ loop; engine cung cấp `step()` và external mutation đi qua typed contract
    do package đăng ký.

## 2. Quy Tắc Vàng (Vòng Ràng Buộc Hệ Thống)

Mọi quy tắc về kiến trúc và chất lượng mã nguồn đều được quy định nghiêm ngặt tại đây. Bạn **BẮT BUỘC** phải đọc trước khi tạo hoặc sửa Crate:
1. 👉 **[Xem Quy Tắc Cấu Trúc Mã Nguồn](file:///c:/Users/abc/.AI/Code/ifol-animation/.agents/rules/00_architecture_rules.md)** (Phụ thuộc vòng, phân chia Crate).
2. 👉 **[Xem Chiến Lược Kiểm Thử Bắt Buộc](file:///c:/Users/abc/.AI/Code/ifol-animation/.agents/rules/01_testing_strategy.md)** (Hướng dẫn tư duy Test-Driven và Edge cases).

### 2.0. Cấm Edit File Bằng Terminal (Công Cụ Sửa Đổi)
*   Tuyệt đối **KHÔNG** sử dụng các lệnh terminal như `echo`, `sed`, `awk` để tạo hoặc sửa file mã nguồn.
*   Việc làm này sẽ khiến IDE (Antigravity/Cursor) không nhận diện được file nào đã thay đổi. Bắt buộc phải dùng các công cụ chuyên dụng (`write_to_file`, `replace_file_content`).

### 2.1. Chuẩn hóa Dự Án Đóng (Self-contained Project)
*   Dự án lưu dưới định dạng bundle `.ifol` gồm manifest, package lock, scene
    records versioned và namespace package; runtime ECS layout không được dump.
*   Engine không hard-code `/assets/`. Package sở hữu dữ liệu của nó dưới virtual
    namespace tương đối và cấm lưu đường dẫn tuyệt đối phụ thuộc máy.

### 2.2. Unified Command Bus (Sự Bình Đẳng UI & AI)
*   Tất cả thay đổi trạng thái từ UI, CLI, MCP hoặc Agent đi qua typed command/
    step-input contract do package đăng ký với `ifol-engine`.
*   Cấm ngặt việc giao diện UI truy cập trực tiếp và thay đổi (mutate) State của ECS. Mọi hành động đều là phát Event / gửi Command.

---

## 3. Lệnh Đọc Bắt Buộc (Mandatory Reading)
Để hiểu triết lý dự án trước khi viết code, agent phải đọc master map và toàn bộ
tài liệu liên quan trực tiếp tới crate/package đang sửa. Khi làm `ifol-engine`,
đọc thêm toàn bộ manual trong `crates/ifol-engine/docs/`.

Hãy bắt đầu bằng việc đọc tấm bản đồ dẫn đường này trước:
👉 **[.agents/design/00_architecture_overview_and_codebase.md](file:///c:/Users/abc/.AI/Code/ifol-animation/.agents/design/00_architecture_overview_and_codebase.md)**

Sau file `00`, dùng Design Map để chọn Core Engine, Application Shell và
Ecosystem contracts bắt buộc cho task. Không dựa vào số lượng file cố định.

---

## 4. Quy Tắc Chuyển Giai Đoạn (Git Milestone Rule)
- Việc `commit` code cho một Giai đoạn CHỈ ĐƯỢC THỰC HIỆN KHI NGƯỜI DÙNG XÁC NHẬN (chẳng hạn khi người dùng gõ `proceed` để bắt đầu Giai đoạn tiếp theo).
- Nếu người dùng chưa xác nhận, tuyệt đối không được tự ý commit, vì họ có thể đang cần kiểm tra và sửa đổi thêm.
- Ngay khi người dùng `proceed` sang Phase mới, Agent phải dùng terminal để `git add .` và `git commit -m "..."` cho Phase cũ (nếu có thay đổi) trước khi bắt tay vào viết code mới.
