# Danh Sách Test Cases Hệ Thống (Cross-Platform Visual Tests)

Tài liệu này lưu trữ định nghĩa 20 Test Cases tiêu chuẩn của dự án `ifol-animation` nhằm kiểm tra độ chính xác của hình ảnh render (pixel-perfect) trên cả Desktop và WebGPU.

## Nhóm 1: Cơ Bản & Nền Tảng (TC01 - TC05)

### TC01 - Empty Render
- **Mục tiêu:** Đo đạc Overhead cơ bản nhất của phần cứng.
- **Kịch bản:** Mở 1 RenderPass với ClearColor (màu xám nhạt `[0.2, 0.2, 0.2, 1.0]`). Không có bất kỳ lệnh vẽ nào.
- **Kỳ vọng đồ thị:** `graphs/tc01_empty.json` (0 Node)
- **Kỳ vọng hình ảnh xuất ra:** Một bức ảnh toàn màu xám nhạt đồng nhất. Không có bất kỳ vật thể hay điểm ảnh lỗi nào. Thời gian compile graph xấp xỉ 0ms.

### TC02 - Single Quad
- **Mục tiêu:** Pipeline cơ bản nhất, test Vertex buffer và Fragment color đơn giản.
- **Kịch bản:** 1 Node vẽ 1 hình chữ nhật lớn màu trắng ở giữa màn hình (Solid color). Nền đen.
- **Kỳ vọng đồ thị:** `graphs/tc02_single_quad.json` (1 Node, 1 DrawCommand)
- **Kỳ vọng hình ảnh xuất ra:** 1 hình chữ nhật chuẩn màu trắng nằm giữa nền đen. Viền cạnh sắc nét (không khử răng cưa).

### TC03 - Z-Buffer Culling
- **Mục tiêu:** Kiểm tra DepthTest (Pipeline's Z-Buffer).
- **Kịch bản:** Vẽ 3 hình chữ nhật có kích thước khác nhau đè lên nhau (Đỏ Z=0.1, Xanh lá Z=0.5, Xanh dương Z=0.9). Hình được Add sau nhưng có Z sâu hơn phải bị culling.
- **Kỳ vọng hình ảnh xuất ra:** Hình Đỏ (Z=0.1 gần nhất) nằm trên cùng che Xanh Lá. Xanh Lá che Xanh Dương. Không có hiện tượng z-fighting.

### TC04 - Alpha Blending & Z-Buffer Interaction
- **Mục tiêu:** Đảm bảo hệ thống xử lý giao thoa đúng giữa đối tượng đục (Opaque) và bán trong suốt (Transparent).
- **Kịch bản:** Một khối đục nằm ở Z=0.5. Một khối trong suốt màu vàng nằm trước mặt (Z=0.2). Một khối trong suốt màu lục nằm phía sau khối đục (Z=0.8).
- **Kỳ vọng hình ảnh xuất ra:** Khối màu Vàng Z=0.2 phải trong suốt và nhìn xuyên thấy khối đục Z=0.5. Khối màu Lục Z=0.8 phải bị khối đục Z=0.5 che khuất hoàn toàn ở vùng giao nhau.

### TC05 - Interleaved Passes
- **Mục tiêu:** Kiểm thử SubGraph và khả năng gom RenderPass.
- **Kịch bản:** Graph A vẽ khối Đỏ ra Offscreen. Graph B lấy Offscreen đó vẽ thêm khối Vàng. Cùng lúc Graph C lấy Offscreen vẽ đè khối Xanh.
- **Kỳ vọng hình ảnh xuất ra:** Output cuối cùng phản ánh kết quả gộp của A, B, C. Đỏ, Vàng, Xanh phải hiển thị đúng thứ tự submit.

---

## Nhóm 2: Cấu Trúc Đồ Thị & Trình Biên Dịch (TC06 - TC11)

### TC06 - Node Garbage Collection
- **Mục tiêu:** Rò rỉ bộ nhớ (Memory Leak) ở cấp độ RenderNodePool.
- **Kịch bản:** Tạo 100 Node, xóa 99 Node (Remove từ Arena). Render Graph chỉ giữ ID của 1 Node duy nhất còn lại.
- **Kỳ vọng hình ảnh xuất ra:** Màn hình chỉ hiển thị kết quả của 1 Node duy nhất.

### TC07 - Deep Recursion SubGraphs
- **Mục tiêu:** Kiểm thử chống tràn Stack và đệ quy đồ thị sâu.
- **Kịch bản:** A lồng B, B lồng C, C lồng D, D lồng E (5 cấp độ SubGraph).
- **Manifest dùng chung:** `shared_assets/manifests/tc07_recursion.json`.
- **Kỳ vọng hình ảnh xuất ra:** Output phải bao gồm kết quả của cấp E (sâu nhất) được render lên nền của D, C, B, A. 

### TC08 - Massive Procedural Instances (10,000)
- **Mục tiêu:** Đánh giá độ trễ của cấu trúc Arena (Node) so với Buffer (DrawCommand).
- **Manifest dùng chung:** `shared_assets/manifests/tc08_massive.json`.
- **Kịch bản:** Một node chứa background blit và một draw command procedural với 10.000 instance hạt bụi màu vàng, cyan và trắng bằng hash trong shader.
- **Kỳ vọng hình ảnh xuất ra:** Màn hình lấm tấm 10,000 điểm trắng/vuông nhỏ phân bố giả ngẫu nhiên.

### TC08.5 - Directional Moonlight Scene
- **Mục tiêu:** Kiểm thử graph nhiều pipeline, texture/sampler contract, ánh sáng định hướng và post-process giữa Desktop/WebGPU.
- **Manifest dùng chung:** `shared_assets/manifests/tc08_5_nightsky.json`.
- **Kịch bản:** Pass `scene` vẽ sky procedural, 100 sao, mặt trăng và 4 lớp mây; pass `final` đọc scene để thực hiện bloom/tone mapping.
- **Kỳ vọng hình ảnh xuất ra:** Mặt trăng ở góc trên trái, sao phân bố trên nền trời đêm, bốn lớp mây có silver lining theo hướng mặt trăng và không có artifact rõ ràng.
- **Kết quả parity hiện tại:** Vision/structural đạt; raw khác 1 byte ở 1 pixel với sai số tối đa `1/255`, xem `reports/tc08_5_nightsky_report.md`.

### TC09 - Pipeline Caching & Bundle Reuse
- **Mục tiêu:** Tính năng tối ưu cốt lõi của Engine (RenderBundle).
- **Manifest dùng chung:** `shared_assets/manifests/tc09_caching.json`.
- **Kịch bản:** Chạy cùng graph một lần cold và 10 lần warm, tái sử dụng graph, pipeline và resource; không rebuild giữa các lần.
- **Kỳ vọng hình ảnh xuất ra:** Hình nền sao và 10.000 hạt giữ nguyên giữa cold/warm và giữa Desktop/WebGPU. Timing warm chỉ là số liệu quan sát, không áp đặt ngưỡng cố định giữa phần cứng.
- **Kết quả parity hiện tại:** Raw parity tuyệt đối; cold/warm output giống nhau ở cả hai môi trường. Xem `reports/tc09_caching_report.md`.

### TC10 - Missing Resources (Edge Case)
- **Mục tiêu:** Ổn định (Zero-Crash) khi thiếu tài nguyên ảnh.
- **Manifest dùng chung:** `shared_assets/manifests/tc10_fallback.json`.
- **Kịch bản:** Gửi `BindGroupHandle(999999)` không tồn tại, xác nhận typed error rồi render fallback magenta.
- **Kỳ vọng hình ảnh xuất ra:** Hình vuông hiển thị màu hồng cánh sen (Magenta) thay vì crash phần mềm.
- **Kết quả parity hiện tại:** Validation Desktop pass, Web mirror cùng error contract; raw fallback parity tuyệt đối. Xem `reports/tc10_fallback_report.md`.

### TC11 - Multi-Viewport Isolation
- **Mục tiêu:** Khả năng chạy song song nhiều Camera / Scene.
- **Manifest dùng chung:** `shared_assets/manifests/tc11_viewport.json`.
- **Kịch bản:** Render hai target offscreen độc lập `400x600`, sau đó compositor ghép thành target `800x600` với divider tại `x=0.5`.
- **Kỳ vọng hình ảnh xuất ra:** Hai nửa trái/phải giữ đúng clear riêng, divider cyan-trắng nằm giữa và không có state leak giữa hai viewport. Xem `reports/tc11_viewport_report.md`.

---

## Nhóm 3: Hiệu Ứng Nâng Cao (TC12 - TC19)

### TC12 - Chroma Key
- **Mục tiêu:** Chroma key nhiều sprite với despill, alpha feather và alpha blending.
- **Kịch bản:** Dùng sky và atlas PNG canonical; render 5 crop sprite theo một graph, áp dụng `chroma_key_cropped.wgsl` với key màu, tolerance và smoothness cố định.
- **Kỳ vọng hình ảnh xuất ra:** Nền hoàng hôn phủ toàn khung; 5 đối tượng đúng vị trí/tỷ lệ, phông xanh bị loại bỏ, viền xanh giảm và không có artifact.

### TC13 - Gaussian Blur & Cinematic Depth of Field (4-Pass Ping-Pong)
- **Mục tiêu:** Kiểm thử graph multi-pass với hai target trung gian ping-pong và phân biệt hậu cảnh blur với tiền cảnh sắc nét.
- **Kịch bản:** Render hậu cảnh rừng canonical và wisps vào `background_a`; chạy Gaussian blur ngang vào `blur_b`, blur dọc trả về `background_a`; blit hậu cảnh đã blur rồi ghép paladin, archer và chest tiền cảnh sắc nét vào `final`.
- **Kỳ vọng hình ảnh xuất ra:** Hậu cảnh rừng được blur mềm theo hai hướng, ba đối tượng tiền cảnh vẫn sắc nét; không có banding, ping-pong state leak hoặc artifact rõ ràng.
- **Hợp đồng parity:** Desktop/Web dùng manifest `shared_assets/manifests/tc13_blur.json`, cùng target `800x600 Rgba8UnormSrgb`, 4 pass và 11 draw command.

### TC14 - Cinematic Color Grading & ACES Filmic Tone Mapping
- **Mục tiêu:** Kiểm thử post-process color grading xác định với exposure, contrast, saturation, temperature, split-toning, ACES Filmic và vignette.
- **Manifest dùng chung:** `shared_assets/manifests/tc14_grading.json`.
- **Kịch bản:** Render scene hoàng hôn canonical trong `scene`, sau đó pass `color_grade` đọc scene và ghi `final` bằng shader `color_grading_filmic.wgsl`.
- **Kỳ vọng hình ảnh xuất ra:** Cảnh có tông vàng ấm, vùng tối pha tím chàm, highlight vàng hổ phách, vignette mềm; foreground còn chi tiết, không bị đen toàn ảnh hay artifact.

### TC15 - Instanced Snow Particle Physics
- **Mục tiêu:** Kiểm thử instanced rendering với chuyển động tuyết xác định theo gravity, wind, rotation và depth.
- **Manifest dùng chung:** `shared_assets/manifests/tc15_snow.json`.
- **Kịch bản:** Một pass vẽ sky, moon, cloud, hai pine, paladin và 200 snow instances bằng `snow_physics_instanced.wgsl` từ input `canonical_particle_snow.png`.
- **Kỳ vọng hình ảnh xuất ra:** Cảnh đêm tuyết không rỗng; 200 hạt tuyết trắng có kích thước/độ mờ theo depth, không có validation error hoặc black output.

### TC16 - 2D SDF Shapes & Vector Graphics
- **Mục tiêu:** Dựng hình vector procedural bằng Signed Distance Field, không phụ thuộc texture.
- **Kịch bản:** Một pass vẽ bốn hình bằng `sdf_shapes.wgsl`: circle, rounded rectangle, ring và triangle; mỗi hình có thông số vị trí, scale, màu, viền, glow và rotation riêng.
- **Kỳ vọng hình ảnh xuất ra:** Bốn hình phân biệt rõ trên nền slate, anti-aliasing mượt, viền/glow đúng mô tả, không có black output hoặc validation error.

### TC17 - Multi-Pass Outline Stroke & Drop Shadow
- **Mục tiêu:** Hậu kỳ outline và drop shadow dựa trên alpha của layer offscreen.
- **Kịch bản:** Pass đầu render paladin, mage và rương vào target trong suốt; pass sau render sky rồi lấy target đó để dò biên 8 hướng, vẽ viền trắng và bóng đổ đen.
- **Kỳ vọng hình ảnh xuất ra:** Nền sky tím/magenta, ba sprite rõ ràng, viền trắng bao quanh và bóng đổ lệch nhẹ, không có black output hoặc validation error.

### TC18 - Video Transition Effects (Glitch)
- **Mục tiêu:** Kiểm thử pipeline dual-texture đọc đồng thời hai cảnh đầu vào và tạo chuyển cảnh glitch xác định.
- **Kịch bản:** Render cảnh A (sky tím + paladin) và cảnh B (sky xanh + mage) vào hai target offscreen; pass thứ ba ghép chúng với progress `0.5`, block shift và RGB split.
- **Kỳ vọng hình ảnh xuất ra:** Kết quả chuyển cảnh glitch giữa hai cảnh, có biến dạng block và quang sai RGB; không có black output hoặc validation error.

### TC19 - Dynamic State Change
- **Mục tiêu:** Thử nghiệm thay đổi Pipeline (BlendMode).
- **Kịch bản:** 3 Node lồng lên nhau với 3 chế độ hòa trộn: Replace, Additive, Multiply.
- **Kỳ vọng hình ảnh xuất ra:** Vùng giao nhau hiển thị các kết quả màu chính xác tương ứng với toán tử blend.

---

## Nhóm 4: Bài Thi Masterpiece Tích Hợp (TC20)

### TC20 - "Anime Scene" Master Compositing
- **Mục tiêu:** Tích hợp cực hạn.
- **Kịch bản:** Background Night Sky + Character Chroma Key (Layer 1) + Snow (Layer 2) + Glow Bloom + Color grading.
- **Kỳ vọng hình ảnh xuất ra:** Một scene hoàn chỉnh: nhân vật đứng dưới trời tuyết đêm, các vùng sáng tỏa hào quang, không có viền xanh xung quanh nhân vật, màu sắc đồng nhất (đã color grade).
