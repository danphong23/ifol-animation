# Danh Sách Master Test Cases (GPU Engine)

Tài liệu này lưu trữ định nghĩa 20 Test Cases tiêu chuẩn của dự án `ifol-animation` nhằm kiểm tra độ chính xác của hình ảnh render, khả năng biên dịch RenderGraph, và hiệu năng (Performance) của lõi GPU.

Mỗi khi thay đổi lõi đồ họa, toàn bộ 20 Test Cases này phải được vượt qua.

---

## Nhóm 1: Cơ Bản & Nền Tảng (TC01 - TC05)

### TC01 - Empty Render
- **Mục tiêu:** Đo đạc Overhead cơ bản nhất của phần cứng.
- **Kịch bản:** Mở 1 RenderPass với ClearColor (màu xám nhạt). Không có bất kỳ lệnh vẽ nào.
- **Kỳ vọng:** Hình ảnh xám. Thời gian compile graph xấp xỉ 0ms.

### TC02 - Single Quad
- **Mục tiêu:** Pipeline cơ bản nhất, test Vertex buffer và Fragment color đơn giản.
- **Kịch bản:** 1 Node vẽ 1 hình chữ nhật lớn ở giữa màn hình (Solid color).
- **Kỳ vọng:** 1 hình chữ nhật chuẩn màu. Render graph có 1 Node, 1 DrawCommand.

### TC03 - Z-Buffer Culling
- **Mục tiêu:** Kiểm tra DepthTest (Pipeline's Z-Buffer).
- **Kịch bản:** Vẽ 3 hình chữ nhật có kích thước khác nhau (Đỏ Z=0.1, Xanh lá Z=0.5, Xanh dương Z=0.9). Hình được Add sau nhưng có Z sâu hơn phải bị culling (không vẽ đè).
- **Kỳ vọng:** Hình Đỏ luôn nằm trên cùng.

### TC04 - Alpha Blending & Z-Buffer Interaction
- **Mục tiêu:** Đảm bảo hệ thống xử lý giao thoa đúng giữa đối tượng đục (Opaque) và bán trong suốt (Transparent).
- **Kịch bản:** Một khối đục nằm ở Z=0.5. Một khối trong suốt màu vàng nằm trước mặt (Z=0.2). Một khối trong suốt màu lục nằm phía sau khối đục (Z=0.8).
- **Kỳ vọng:** Khối trong suốt Z=0.2 blend với khối đục. Khối trong suốt Z=0.8 bị khối đục che khuất hoàn toàn (Z-tested).

### TC05 - Interleaved Passes
- **Mục tiêu:** Kiểm thử SubGraph và khả năng gom RenderPass.
- **Kịch bản:** Graph A vẽ khối Đỏ ra Offscreen. Graph B lấy Offscreen đó vẽ thêm khối Vàng. Cùng lúc Graph C lấy Offscreen vẽ đè khối Xanh.
- **Kỳ vọng:** Output cuối cùng phản ánh thứ tự RenderPass chính xác mà không bị mất dữ liệu.

---

## Nhóm 2: Cấu Trúc Đồ Thị & Trình Biên Dịch (TC06 - TC11)

### TC06 - Node Garbage Collection
- **Mục tiêu:** Rò rỉ bộ nhớ (Memory Leak) ở cấp độ RenderNodePool.
- **Kịch bản:** Tạo 100 Node, xóa 99 Node (Remove từ Arena). Render Graph chỉ giữ ID của 1 Node duy nhất còn lại.
- **Kỳ vọng:** Node cuối vẽ đúng. Arena len đếm được số lượng chính xác, không bị rác.

### TC07 - Deep Recursion SubGraphs
- **Mục tiêu:** Kiểm thử chống tràn Stack và đệ quy đồ thị sâu.
- **Kịch bản:** A lồng B, B lồng C, C lồng D, D lồng E (5 cấp độ SubGraph).
- **Manifest dùng chung:** `shared_assets/manifests/tc07_recursion.json`.
- **Kỳ vọng:** Output phải bao gồm cả hiệu ứng của 5 cấp gộp lại.

### TC08 - Massive Procedural Instances (10,000)
- **Mục tiêu:** Đánh giá độ trễ của cấu trúc Arena (Node) so với Buffer (DrawCommand).
- **Manifest dùng chung:** `shared_assets/manifests/tc08_massive.json`.
- **Kịch bản:** 
  - SubGraph 1: `1 Node` chứa mảng `10,000 DrawCommand`.
  - SubGraph 2: `10,000 Nodes` trong Arena, mỗi Node chứa `1 DrawCommand`.
- **Kỳ vọng:** Đo lường Overhead của việc look-up Arena. Thời gian compile của SubGraph 2 phải nằm trong mức cho phép (Dưới 5ms).

### TC08.5 - Directional Moonlight Scene
- **Mục tiêu:** Kiểm thử graph hai pass, nhiều pipeline, sampler/texture contract và post-process bloom trên Desktop/WebGPU.
- **Manifest dùng chung:** `shared_assets/manifests/tc08_5_nightsky.json`.
- **Kịch bản:** `scene` gồm sky procedural, 100 sao, mặt trăng và 4 lớp mây; `final` áp dụng bloom/tone mapping.
- **Kỳ vọng:** Hai môi trường dùng cùng fingerprint graph và có cùng bố cục/layer; report phải ghi raw hash, diff và cold/warm timing riêng.

### TC09 - Pipeline Caching & Bundle Reuse
- **Mục tiêu:** Tính năng tối ưu cốt lõi của Engine (RenderBundle).
- **Manifest dùng chung:** `shared_assets/manifests/tc09_caching.json`.
- **Kịch bản:** Chạy cùng graph một lần cold và 10 lần warm, tái sử dụng graph, pipeline và resource, không rebuild giữa các lần.
- **Kỳ vọng:** Cold/warm output giữ nguyên; report ghi riêng timing Desktop/WebGPU và raw parity. Không dùng ngưỡng thời gian tuyệt đối vì phụ thuộc adapter/backend.

### TC10 - Missing Resources (Edge Case)
- **Mục tiêu:** Ổn định (Zero-Crash).
- **Manifest dùng chung:** `shared_assets/manifests/tc10_fallback.json`.
- **Kịch bản:** Đăng ký DrawCommand với `BindGroupHandle(999999)` sai, xác nhận typed error rồi chạy fallback graph.
- **Kỳ vọng:** Desktop trả `RenderGraphValidationError::MissingBindGroup` không panic; Web giữ cùng contract mirror; fallback output magenta giống nhau.

### TC11 - Multi-Viewport Isolation
- **Mục tiêu:** Khả năng chạy song song nhiều Camera / Scene trên 1 Arena Pool.
- **Manifest dùng chung:** `shared_assets/manifests/tc11_viewport.json`.
- **Kịch bản:** Render hai target `400x600` độc lập rồi ghép bằng một split compositor vào target `800x600`.
- **Kỳ vọng:** Hai viewport giữ đúng nội dung riêng, divider xác định và không có state leak; report ghi raw parity Desktop/WebGPU.

---

## Nhóm 3: Hiệu Ứng Nâng Cao (TC12 - TC19)

### TC12 - Chroma Key
- **Mục tiêu:** Chroma key nhiều sprite với despill, alpha feather và alpha blending.
- **Kịch bản:** Dùng sky và atlas PNG canonical; render 5 crop sprite theo một graph, áp dụng `chroma_key_cropped.wgsl` với key màu, tolerance và smoothness cố định.
- **Kỳ vọng:** Nền hoàng hôn phủ toàn khung; 5 đối tượng đúng vị trí/tỷ lệ, phông xanh bị loại bỏ và alpha feather không tạo artifact.

### TC13 - Gaussian Blur & Cinematic Depth of Field (4-Pass Ping-Pong)
- **Mục tiêu:** Kiểm thử graph multi-pass với hai target trung gian ping-pong và phân biệt hậu cảnh blur với tiền cảnh sắc nét.
- **Kịch bản:** `background_scene` → `blur_horizontal_pass` → `blur_vertical_pass` → `final_composite`; hậu cảnh rừng/wisps được blur qua `background_a` và `blur_b`, sau đó ghép ba sprite tiền cảnh sắc nét.
- **Kỳ vọng:** Cùng manifest, 4 pass, 11 draw command; hậu cảnh blur hai hướng, foreground sắc nét, không có banding hoặc ping-pong state leak.

### TC14 - Cinematic Color Grading & ACES Filmic Tone Mapping
- **Mục tiêu:** Kiểm thử post-process color grading xác định với exposure, contrast, saturation, temperature, split-toning, ACES Filmic và vignette.
- **Manifest dùng chung:** `shared_assets/manifests/tc14_grading.json`.
- **Kịch bản:** `scene_pass` render scene hoàng hôn, `grading_pass` đọc scene và ghi `final` bằng `color_grading_filmic.wgsl`.
- **Kỳ vọng:** Cùng manifest và 2 pass; tông vàng ấm, shadow tím chàm, highlight vàng hổ phách, vignette mềm và không có artifact.

### TC15 - Instanced Snow Particle Physics
- **Mục tiêu:** Kiểm thử instanced rendering với chuyển động tuyết xác định theo gravity, wind, rotation và depth.
- **Manifest dùng chung:** `shared_assets/manifests/tc15_snow.json`.
- **Kịch bản:** Một node và một pass vẽ sky, moon, cloud, hai pine, paladin và 200 snow instances bằng `snow_physics_instanced.wgsl`.
- **Kỳ vọng:** Desktop/Web dùng cùng fingerprint, 7 draw command và 200 instances; output không rỗng, không validation error, cold/warm không đổi.

### TC16 - 2D SDF Shapes & Vector Graphics
- **Mục tiêu:** Dựng hình vector procedural bằng Signed Distance Field, không phụ thuộc texture.
- **Kịch bản:** Một pass vẽ bốn hình bằng `sdf_shapes.wgsl`: Circle, Rounded Rect, Neon Ring và Triangle; mỗi hình có màu, viền, glow, scale và rotation riêng.
- **Kỳ vọng:** Bốn hình rõ ràng, anti-aliasing mượt, viền/glow đúng thiết kế, không có black output hoặc validation error.

### TC17 - Luma Masking
- **Mục tiêu:** Trích xuất kết quả SubGraph làm mặt nạ Alpha.
- **Kịch bản:** Node A vẽ ngôi sao (đủ màu). Node B vẽ ảnh nhân vật, dùng hàm Shader trích lấy ảnh sao làm mask.
- **Kỳ vọng:** Nhân vật bị cắt gọn vào trong hình khối ngôi sao.

### TC18 - Color Grading
- **Mục tiêu:** Post-processing Pipeline.
- **Kịch bản:** Nhận ảnh đầu vào, shader chỉnh sửa Brightness +20%, Contrast +1.5, Saturation (Desaturated/Trắng đen).
- **Kỳ vọng:** Màu sắc chính xác như mong muốn.

### TC19 - Dynamic State Change
- **Mục tiêu:** Thử nghiệm thay đổi Pipeline liên tục.
- **Kịch bản:** Node 1 (Blend Replace), Node 2 (Blend Additive), Node 3 (Blend Multiply). 
- **Kỳ vọng:** RenderBundle tự động cache và switch pipeline mà không lỗi rác màn hình.

---

## Nhóm 4: Bài Thi Masterpiece Tích Hợp (TC20)

### TC20 - "Anime Scene" Master Compositing
- **Mục tiêu:** Tích hợp mọi tinh hoa đồ họa và logic của Engine vào 1 scene hoàn chỉnh. Chứng minh hệ thống chịu tải và tương tác hoàn hảo.
- **Kịch bản (Từ dưới lên):**
  1. `Background Node`: Đọc ảnh nền Bầu Trời Đêm Anime thực tế.
  2. `Character Node`: Đọc ảnh nhân vật phông xanh thật. Qua Pipeline Chroma Key bóc nền. Nằm chồng lên Lớp 1 (Z-Culling).
  3. `Snow Node`: Particle Tuyết 10,000 hạt (Instancing) phủ tràn màn hình (Alpha Blend).
  4. `Glow Compositing Node`: Lấy Offscreen của 3 bước trên, Bloom vùng sáng và add ngược lại.
  5. `Color Grading Node`: Bước Post-Process cuối cùng làm không khí lạnh/xanh.
- **Kỳ vọng:** Một khung cảnh đẹp siêu thực, kiến trúc Graph dày dặn (có subgraph, multi-pass), render chính xác từng điểm ảnh.
