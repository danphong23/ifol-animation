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
* **Phiên dịch ra Render Graph:** ECS đẩy lệnh vẽ Cái Cửa vào Graph, đính kèm Tọa Độ Tuyệt Đối.

#### Loại 2: Lồng Nhau Về Hiệu Ứng Pixel (Project lồng Project / Group Alpha)
*Ví dụ: Bạn có 1 Group chứa 3 hình tròn xếp chồng lên nhau. Bạn giảm Alpha của toàn bộ Group xuống 50%. Nếu bạn dùng cách Kế thừa Toán Học (giảm 50% từng hình tròn), thì phần giao nhau giữa 3 hình tròn sẽ hiện ra nét cắt xuyên thấu! Để trông như một Group thực thụ, 3 hình tròn phải vẽ đè lên nhau ở 100% Alpha, sau đó MỚI làm mờ 50% cả cụm.*
* **Cách ECS xử lý:** ECS không dùng toán học. Nó tạo ra một **`SubGraph` (Nhóm)** trong Render Graph.
* **Phiên dịch ra Render Graph:** Đồ thị lồng nhau sẽ được ECS giữ nguyên cấu trúc đệ quy:
  - Nó tạo `SubGraph_Group`, khai báo cờ `post_pipeline = Alpha 50%`.
  - Nhét lệnh vẽ 3 hình tròn vào trong `SubGraph` đó.
  - Nhét `SubGraph_Group` vào Màn Hình Chính.
  - Gửi nguyên cái Cây đệ quy này cho `ifol-gpu`.

---

## 2. Minh Họa Cấu Trúc Đệ Quy Của Render Graph

Khi ECS đụng phải một Composition (Project lồng Project), nó sẽ sinh ra một Đồ thị như sau:

```mermaid
graph TD
    subgraph Render Graph (Cấu Trúc Đệ Quy đẩy xuống ifol-gpu)
        Root[Root Graph: Màn hình chính]
        
        Cmd1(DrawCommand: Vẽ Bầu Trời)
        Root --> Cmd1
        
        SubGraph1((SubGraph: Project B <br> Post-Shader: Blur/Đảo Màu))
        Root --> SubGraph1
        
        Cmd2(DrawCommand: Vẽ Cây)
        Cmd3(DrawCommand: Vẽ Cỏ)
        
        SubGraph1 --> Cmd2
        SubGraph1 --> Cmd3
    end
```

### 3. DrawCacheComponent Làm Việc Ra Sao?
Thay vì phải tính toán lại toàn bộ cái Cây này ở mỗi Frame, các Entity dạng Group/Project sẽ có `DrawCacheComponent`. Component này chứa sẵn cấu trúc `SubGraph` của riêng nó.
Hệ thống chỉ việc "ráp" các khối lego `SubGraph` lại với nhau thành một cái Cây Render Graph to bự và ném cho `ifol-gpu` xử lý.
Engine `ifol-gpu` thông minh sẽ tự cấp phát Texture trung gian cho các `SubGraph` và vẽ mọi thứ đúng thứ tự, đồng thời Cache lại từng nhánh bằng `RenderBundle` để đạt hiệu năng tối đa.
