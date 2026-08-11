# Mảnh Ghép Cuối: Quá Trình Phiên Dịch (Translation Pipeline)

Tài liệu này đặc tả cơ chế "Phiên dịch" — Cây cầu nối giữa hệ thống **ECS Logic (Cây Phân Cấp/Cha Con)** và hệ thống **GPU Engine (ifol-gpu)**.

---

## 1. Bản Chất Quá Trình Phiên Dịch

Quá trình dịch từ Scene sang lệnh vẽ thực chất **là một chuỗi các System nằm trong ECS**.
Nhiệm vụ của ECS là duyệt qua Cây phân cấp (Scene Graph), tính toán tọa độ tuyệt đối, và đóng gói cấu trúc đó lại thành một **Render Graph Đệ Quy** (có chứa các `SubGraph` hay còn gọi là Pre-comp/Nhóm).

### Làm Sao Xử Lý Sự "Lồng Nhau"?

Có 2 cấp độ "Lồng nhau" trong Scene Graph:

#### Loại 1: Lồng Nhau Về Dữ Liệu (Kế thừa Toán Học)
*Ví dụ: Bạn có 1 Ngôi Nhà (Cha) chứa 1 Cái Cửa (Con). Bạn di chuyển Ngôi Nhà, Cái Cửa phải chạy theo.*
* **Cách ECS xử lý:** ECS duyệt cây từ trên xuống. Nó nhân ma trận tọa độ của Ngôi Nhà với Cái Cửa -> Ra Tọa Độ Tuyệt Đối.
* **Phiên dịch ra Render Graph:** ECS đẩy lệnh vẽ Cái Cửa vào `DrawBatch`, đính kèm Tọa Độ Tuyệt Đối (qua Uniform Ring Buffer).

#### Loại 2: Lồng Nhau Về Hiệu Ứng Pixel (Project lồng Project / Group Alpha)
*Ví dụ: Bạn có 1 Group chứa 3 hình tròn xếp chồng lên nhau. Bạn giảm Alpha của toàn bộ Group xuống 50%. Nếu bạn dùng cách Kế thừa Toán Học (giảm 50% từng hình tròn), thì phần giao nhau giữa 3 hình tròn sẽ hiện ra nét cắt xuyên thấu! Để trông như một Group thực thụ, 3 hình tròn phải vẽ đè lên nhau ở 100% Alpha, sau đó MỚI làm mờ 50% cả cụm.*
* **Cách ECS xử lý:** ECS tạo ra một **`SubGraph` (Nhóm)** trong Render Graph.
* **Phiên dịch ra Render Graph:** Đồ thị lồng nhau sẽ được ECS giữ nguyên cấu trúc đệ quy:
  - Nó tạo `SubGraph_Group` với graph con có target là Offscreen.
  - Nhét lệnh vẽ 3 hình tròn vào `DrawBatch` bên trong graph con.
  - Trong danh sách `commands` của SubGraph, ECS tạo `DrawCommand` áp Shader Alpha 50% lên kết quả Offscreen, vẽ lên target của Graph cha.
  - Gửi nguyên cái Cây đệ quy này cho `ifol-gpu`.

---

## 2. Minh Họa Cấu Trúc Đệ Quy Của Render Graph

Khi ECS đụng phải một Composition (Project lồng Project), nó sẽ sinh ra một Đồ thị như sau:

```text
Root Graph (Target: Screen)
│
├── DrawBatch: [DrawCommand: Vẽ Bầu Trời]
│
└── SubGraph "Project B"
    ├── graph con (Target: Offscreen 800x600):
    │   └── DrawBatch: [
    │         DrawCommand: Vẽ Cây
    │         DrawCommand: Vẽ Cỏ
    │       ]
    └── commands: [
          DrawCommand(pipeline: blur_shader, bind: [offscreen_tex], action: Procedural(3))
        ]
        ↑ Lấy Offscreen, áp Blur, in lên Screen
```

### 3. DrawCacheComponent Làm Việc Ra Sao?
Thay vì phải tính toán lại toàn bộ cái Cây này ở mỗi Frame, các Entity dạng Group/Project sẽ có `DrawCacheComponent`. Component này chứa sẵn cấu trúc `SubGraph` của riêng nó.
Hệ thống chỉ việc "ráp" các khối lego `SubGraph` lại với nhau thành một cái Cây Render Graph to bự và ném cho `ifol-gpu` xử lý.
Engine `ifol-gpu` sẽ tự cấp phát Texture trung gian cho các `SubGraph` (thông qua Texture Pool), vẽ mọi thứ đúng thứ tự, và trong tương lai Cache lại từng nhánh bằng `RenderBundle` để đạt hiệu năng tối đa.
