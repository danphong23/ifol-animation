# Giao Tiếp: Svelte UI, ECS Singleton & AI (MCP)

Tài liệu này định nghĩa cách lớp Vỏ Giao Diện (UI) và Hệ thống Trí Tuệ Nhân Tạo (AI Agent) tương tác với lõi ECS. Điểm mấu chốt là **Sự Bình Đẳng (Parity)** và **Đồng Bộ Thời Gian Thực (Live Sync)**.

---

## 1. Trái Tim Singleton (Core State)
*   Khi phần mềm khởi động, ECS World và GPU Engine được bật lên tạo thành một **Background Service (Singleton)**.
*   Nó nắm giữ toàn bộ trạng thái (State) của project: Có bao nhiêu layer, đang ở giây thứ mấy, màu sắc là gì.
*   Lớp giao diện Svelte UI hoàn toàn "ngu ngốc" (Dumb View). Nó không tự ý lưu trữ dữ liệu project. Nó chỉ là một lớp hiển thị những gì ECS bảo nó hiển thị.

## 2. Command Bus (Cổng Nhận Lệnh Độc Nhất)
Để thay đổi bất kỳ thứ gì trong dự án, hệ thống bắt buộc phải đi qua một cổng kiểm duyệt gọi là `CommandBus`.

### 2.1. Cấu trúc Lệnh (Command)
Các lệnh được định nghĩa dưới dạng JSON chuẩn mực. Ví dụ:
```json
{
  "action": "AddEntity",
  "payload": { "type": "Shape", "color": "red" }
}
```

### 2.2. Sự Bình Đẳng Giữa Người & Máy
*   **Người Dùng Thật (UI):** Khi user click nút "Add Shape" trên giao diện Svelte, Svelte sẽ ném cục JSON trên vào `CommandBus`.
*   **AI Agent (MCP):** Khi AI muốn thao tác, nó cũng thông qua Model Context Protocol (MCP) ném đúng cục JSON y hệt vào `CommandBus`.
👉 ECS hoàn toàn không phân biệt lệnh này do ai gửi. Cả UI và MCP đều tuân thủ 1 bộ API Docs duy nhất.

## 3. Đồng Bộ Thời Gian Thực (Live Synchronization)
Vậy làm sao để khi AI (MCP) thêm một hình khối, giao diện của User lập tức xuất hiện khối đó mà không cần tải lại trang?

**Cơ chế Pub/Sub (Event Emitter):**
1.  MCP gửi lệnh `AddEntity` vào Command Bus.
2.  ECS Singleton nhận lệnh, xử lý thay đổi dữ liệu trong RAM.
3.  Xử lý xong, ECS **phát ra một Event Broadcast** (Ví dụ: `Event: StateChanged`).
4.  Giao diện Svelte UI luôn luôn lắng nghe (Subscribe) các Event này.
5.  Ngay khi nhận Event, Svelte UI gọi hàm lấy dữ liệu mới nhất từ ECS và cập nhật Reactivity (`$state` trong Svelte 5).
6.  **Kết quả:** User thấy AI Agent điều khiển phần mềm trực tiếp trên màn hình của mình (Zero-latency AI Pairing).
