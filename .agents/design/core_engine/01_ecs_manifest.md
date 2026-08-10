# Lõi ECS: Kiến Trúc Khung Xương (Framework Architecture)

Tài liệu này định nghĩa bản chất và cơ chế vận hành của hệ thống ECS trong `ifol-animation`. Tài liệu **chỉ mô tả khung xương thuần túy** — không định nghĩa bất kỳ Component hay System cụ thể nào (ngoại trừ Render Component/System vì nó là phần bắt buộc của khung xương), vì phần còn lại chỉ là phần mở rộng được đắp thêm vào sau.

---

## 1. Bản Chất Của ECS

ECS là một cỗ máy xử lý dữ liệu theo pha (Phase Pipeline). Mục đích cuối cùng duy nhất của nó là: **Biến đổi dữ liệu phức tạp của Scene (phân cấp cha-con, tọa độ tương đối, thuộc tính vật lý,...) thành các danh sách Draw Call phẳng gửi cho GPU Engine vẽ.**

Bản thân ECS không quan tâm Component là gì, System là gì. Nó chỉ quan tâm:
*   Các System có tạo ra hoặc biến đổi Draw Call cho Entity hay không.
*   Ở phase render cuối cùng, những Entity nào đã đăng ký yêu cầu render (mang `RenderComponent`) — và mỗi Entity đích (target) có Draw Call gì thì GPU vẽ cái đó.

---

## 2. Các Thành Phần Khung Xương

### 2.1. Entity
Chỉ là một ID định danh độc nhất. Bản thân nó rỗng hoàn toàn, không chứa dữ liệu hay logic.

### 2.2. Component
Là các túi dữ liệu thuần túy (Pure Data Struct) được gắn vào Entity. Khung xương ECS không quy định Component phải chứa gì — đó là việc của các tài liệu mở rộng khác.

Cho dù một Entity không có bất kỳ Component nào khác ngoài Component sinh đồ họa đơn giản, hệ thống vẫn có thể render được nó.

### 2.3. System
Là các đơn vị logic xử lý dữ liệu thuần túy, hoạt động theo pha (Phase). Mỗi System quét qua các Entity có tập Component mà nó quan tâm, đọc và biến đổi dữ liệu. Khung xương ECS không quy định System nào phải tồn tại (ngoại trừ Render System).

### 2.4. World
Trình quản lý trung tâm chứa toàn bộ Entity và các bảng ánh xạ Component của chúng. World cũng chịu trách nhiệm chạy các System theo đúng thứ tự pha đã đăng ký.

---

## 3. Draw Call — Sản Phẩm Của Pipeline ECS

Draw Call không phải là thứ được khai báo cứng khi tạo Entity. Nó được **sinh ra trong quá trình ECS chạy các phase**, dưới dạng dữ liệu cache hoặc component runtime gắn vào Entity.

Khi các System chạy qua từng phase (tính tọa độ, phân cấp cha-con, animation,...), chúng cập nhật và sinh ra Draw Call cho các Entity có component sinh đồ họa. Draw Call này là kết quả tính toán cuối cùng, đã được phẳng hóa hoàn toàn:
*   **Không còn khái niệm Entity, quan hệ cha-con, hay dữ liệu vật lý/xương.**
*   **Chỉ còn:** Shader ID, Texture Key(s), tọa độ pixel, kích thước, layer, và mảng số thực Uniform thô.

---

## 4. Cơ Chế Vận Hành Theo Pha (Phase Pipeline)

```text
[Phase 1..N: Các System tính toán]
    Ví dụ: Tính tọa độ tuyệt đối, phân cấp cha-con, nội suy animation,...
    Kết quả: Draw Call được sinh ra / cập nhật dưới dạng cache runtime trên các Entity.
                    │
                    ▼
[Phase Render: Render System quét toàn bộ RenderComponent]
    Tìm tất cả Entity mang RenderComponent trong World.
    Mỗi RenderComponent trỏ đến Entity đích cần render.
    Với mỗi Entity đích:
      - Kiểm tra xem Entity đó có Draw Call (đã sinh ở các phase trước) hay không.
      - Nếu có → thu thập Draw Call phẳng tương ứng.
    Tổng hợp tất cả Draw Call từ mọi RenderComponent, tối ưu hóa
    (gom nhóm tài nguyên dùng chung, loại bỏ render trùng lặp).
                    │
                    ▼
[Gửi cho GPU Engine]
    GPU Engine nhận toàn bộ danh sách Draw Call phẳng đã tối ưu
    và vẽ 1 lần duy nhất.
```

---

## 5. Các Component Bắt Buộc Của Khung Xương

Để đảm bảo kiến trúc nhất quán tuyệt đối (mọi thứ đều là Entity + Component), sự giao tiếp giữa đồ họa và hệ thống ECS phải thông qua 2 Component cốt lõi sau:

### 5.1. `DrawCacheComponent`
Là cái rương chứa kết quả tính toán đồ họa của một Entity.
*   **Chức năng:** Bất kỳ Entity nào có khả năng hiển thị (Shape, Image, Camera, Scene con) đều phải được các System đồ họa "nhào nặn" và nén kết quả lại thành một **Render Graph**. Render Graph này sau đó được lưu vào `DrawCacheComponent`.
*   **Tính chất:** Nhờ có Component này, hệ thống có thể tái sử dụng (Cache) kết quả của frame trước nếu Entity không có sự thay đổi, bỏ qua khối lượng tính toán đồ họa khổng lồ.

### 5.2. `RenderRequestComponent`
Là điểm kích hoạt yêu cầu vẽ, gắn vào một Entity rỗng đại diện cho một Viewport/Màn hình Editor. Nó chứa:
*   **Target Entity ID:** Chứa ID trỏ đến một Entity đích (Ví dụ: Camera) mà nó muốn xuất ra màn hình.
*   **Output Config (Đích đến):** Chứa thông tin về nơi GPU sẽ in kết quả ra. Có thể là một **OS Window Surface Handle** (để vẽ thẳng lên màn hình) hoặc một **Offscreen Texture ID** (để trả về RAM cho UI tự xử lý).
*   **Cách hoạt động:** Ở phase cuối cùng, Render System sẽ quét World tìm `RenderRequestComponent`. Với mỗi yêu cầu, nó tìm đến Entity đích, móc cái `DrawCacheComponent` (chứa Render Graph) của Entity đó ra, đính kèm cái `Output Config` và gửi thẳng xuống GPU Engine.

### 5.3. Ví dụ ứng dụng thực tế
*   **Editor đơn viewport:** Tạo 1 Entity rỗng mang `RenderRequestComponent` trỏ đến Camera Entity chính. Render System quét thấy, lấy Render Graph trong `DrawCacheComponent` của Camera đó gửi cho GPU.
*   **Editor đa viewport:** Tạo 2 Entity rỗng mang 2 `RenderRequestComponent` trỏ đến 2 Camera khác nhau. Hệ thống thu thập cả 2 Render Graph, gom lại tối ưu rồi gửi GPU.
*   **Preview 1 bức ảnh:** `RenderRequestComponent` trỏ thẳng đến Entity mang ảnh. Hệ thống lấy Render Graph đơn giản (chỉ 1 lệnh vẽ) của bức ảnh đó đem đi render.

---

## 6. Tổng Kết Kiến Trúc

*   **Entity, Component, System**: Khung xương chung, không quy định nội dung cụ thể.
*   **DrawCacheComponent & RenderRequestComponent**: Cặp bài trùng bắt buộc duy nhất của khung xương. Hệ thống các System tự tính toán, tự đóng gói đồ họa vào Cache, và chỉ gửi cho GPU khi có Request.
*   **GPU Engine**: Hoàn toàn mù quáng, chỉ nhận Render Graph phẳng và vẽ theo yêu cầu.
