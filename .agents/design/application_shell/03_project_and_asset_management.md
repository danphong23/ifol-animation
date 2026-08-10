# Quản Lý Project & Asset Đa Nền Tảng

Bài toán hóc búa nhất của đa nền tảng là: Web không có đường dẫn ổ cứng (`C:/...`), trong khi Desktop thì có. Làm sao để một Project mở trên App và Web đều hoạt động? Câu trả lời là: **Gói độc lập (Bundle) và Đường dẫn tương đối (Relative Path)**.

---

## 1. Cấu Trúc Project Tự Chứa (Self-Contained Bundle)
Thay vì file project chỉ chứa chữ (JSON), khi người dùng ấn "Save", phần mềm sẽ tạo ra một gói (Bundle) với định dạng `.ifol`. Thực chất `.ifol` là một thư mục nén `.zip` với cấu trúc như sau:

```text
my_animation.ifol (ZIP archive)
├── project.json      (Lưu toàn bộ Entity, Component của ECS)
└── assets/           (Thư mục chứa toàn bộ tài nguyên dùng trong project)
    ├── video_01.mp4
    └── logo_02.png
```

## 2. Hệ Thống Đường Dẫn Tương Đối
*   Khi người dùng kéo thả 1 file `C:/Users/abc/Downloads/logo_02.png` vào phần mềm, Asset Manager **KHÔNG** ghi lại đường dẫn ổ cứng này.
*   Nó sẽ âm thầm **copy** file đó vào thư mục `assets/` của project hiện tại.
*   Trong bộ nhớ của ECS, nó chỉ lưu đường dẫn tương đối: `./assets/logo_02.png`.
*   👉 **Kết quả:** Bạn gửi file `.ifol` này cho máy Mac, máy Linux, hay upload lên Web, project vẫn chạy bình thường vì mọi tài nguyên đều nằm bên trong nó, không bị đứt gãy đường dẫn (Broken Links).

## 3. Hệ Thống VFS (Virtual File System)
Làm sao lõi ECS biết đọc file `./assets/logo_02.png` ở đâu khi chạy đa nền tảng? Chúng ta sử dụng một lớp trừu tượng VFS ở giữa:

### 3.1. Chế độ Desktop App (Tauri)
*   VFS dịch `./assets/...` thành đường dẫn thư mục tạm thời trên ổ cứng mà Tauri vừa bung nén file `.ifol` ra.
*   ECS dùng hàm đọc file hệ điều hành chuẩn (`std::fs`) cực kỳ nhanh.

### 3.2. Chế độ Web App
*   Trình duyệt giải nén file `.ifol` (bằng JS zip library) vào bộ nhớ RAM hoặc IndexedDB.
*   File ảnh/video được hệ thống Web tạo thành các Virtual URL (dạng `blob://...`).
*   VFS ở lõi WASM dịch `./assets/...` thành các đường dẫn `blob://...` này. 
*   GPU Engine và FFmpeg WASM gọi vào các đường link ảo này để đọc byte mà không cần quan tâm nó không nằm trên ổ cứng vật lý.
