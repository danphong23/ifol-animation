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

### TC17 - Multi-Pass Outline Stroke & Drop Shadow
- **Mục tiêu:** Hậu kỳ outline và drop shadow dựa trên alpha của layer offscreen.
- **Kịch bản:** Pass đầu render paladin, mage và rương vào layer trong suốt; pass sau render sky rồi áp dụng shader dò biên 8 hướng để tạo viền trắng và bóng đổ đen.
- **Kỳ vọng:** Ba sprite rõ ràng trên nền sky tím/magenta, viền trắng và bóng đổ đúng mô tả.

### TC18 - Video Transition Effects (Glitch)
- **Mục tiêu:** Kiểm thử pipeline dual-texture đọc đồng thời hai cảnh đầu vào và tạo chuyển cảnh glitch xác định.
- **Kịch bản:** Render cảnh A (sky tím + paladin) và cảnh B (sky xanh + mage) vào hai target offscreen; pass thứ ba ghép chúng với progress `0.5`, block shift và RGB split.
- **Kỳ vọng:** Kết quả chuyển cảnh glitch giữa hai cảnh, có biến dạng block và quang sai RGB; không có black output hoặc validation error.

### TC19 - Audio-Reactive Spectrum Visualizer
- **Mục tiêu:** Kiểm thử truyền mảng 16 dải tần qua uniform buffer và dựng phổ âm thanh neon xác định.
- **Kịch bản:** Shader nhận 16 frequency bands được đóng gói thành bốn `vec4`, màu cyan cơ sở, thời gian cố định và texture noise canonical để tạo nền grid, cột phổ, glow và peak line.
- **Kỳ vọng:** Nền grid neon cyan/tím với 16 cột có chiều cao khác nhau, glow và peak line rõ; không có black output hoặc validation error.

---

## Nhóm 4: Bài Thi Masterpiece Tích Hợp (TC20)

### TC20 - 3D Perspective Projection & Card Flip (2.5D)
- **Mục tiêu:** Kiểm thử truyền ma trận model-view-projection cố định vào WGSL và render sprite 2D trong phối cảnh 3D.
- **Kịch bản:** Một sprite paladin canonical được crop, xoay theo MVP với phối cảnh camera, đồng thời áp dụng chroma-key/despill.
- **Kỳ vọng:** Sprite nghiêng theo phối cảnh trên nền xám, crop và alpha đúng, không có black output hoặc validation error.

### TC21 - SDF Masking & Chroma Key
- **Mục tiêu:** Kiểm thử mask tròn SDF kết hợp chroma-key/despill cho avatar.
- **Kịch bản:** Crop avatar paladin canonical, áp dụng transform cố định, lọc phông xanh rồi nhân alpha với mask tròn mềm trong local space.
- **Kỳ vọng:** Avatar nằm trong vùng mask tròn, nền xám bên ngoài được giữ nguyên, không có black output hoặc validation error.

### TC22 - Hardware Instancing (Props)
- **Mục tiêu:** Kiểm thử 100 instance phần cứng bằng một draw command.
- **Kịch bản:** Dùng chung một crop sprite canonical, shader tạo vị trí, scale và rotation xác định theo `instance_index`, có sửa aspect ratio và loại phông xanh.
- **Kỳ vọng:** 100 prop nhỏ phân bố xác định trên nền xanh đậm, không có black output hoặc validation error.

### TC23 - Palette Swap (HSV Shift)
- **Mục tiêu:** Kiểm thử thay bảng màu HSV từ hồng sang cyan nhưng giữ shading/highlight.
- **Kịch bản:** Crop sprite canonical, dùng `color_replace.wgsl` để tính khoảng cách HSV, dịch hue/saturation, giữ value và loại phông xanh.
- **Kỳ vọng:** Giáp và chi tiết chuyển sang cyan, shading/highlight giữ nguyên, không có output đen hoặc validation error.

### TC24 - Vertex Deformation (Wind/Sway)
- **Mục tiêu:** Kiểm thử biến dạng đỉnh của sprite theo hiệu ứng gió xác định.
- **Kịch bản:** Dùng `distortion.wgsl` để neo phần dưới sprite và uốn phần trên theo `sin(time * frequency) * amplitude`.
- **Kỳ vọng:** Sprite nằm trên nền xám, phần dưới được neo, phần trên uốn theo gió; không có black output hoặc validation error.

### TC25 - Fake Rim Lighting & Drop Shadow
- **Mục tiêu:** Kiểm thử rim light và drop shadow bằng hai instance trong một draw command.
- **Kịch bản:** Instance 0 dịch sprite để tạo bóng đen bán trong suốt; instance 1 render sprite chính và tính viền sáng vàng bằng cách dò alpha lân cận.
- **Kỳ vọng:** Sprite chính có viền sáng vàng và bóng đổ lệch phía sau, không có black output hoặc validation error.

### TC26 - Glitch & Chromatic Aberration
- **Mục tiêu:** Kiểm thử glitch theo dải ngang và tách kênh RGB với hash xác định giữa backend.
- **Kịch bản:** Dùng `glitch.wgsl` tạo block shift theo integer hash từ `time`, sau đó lấy mẫu lệch cho kênh đỏ/xanh và loại phông xanh.
- **Kỳ vọng:** Sprite vẫn nhận diện được nhưng có dải glitch ngang và RGB split rõ, không có black output hoặc validation error.

### TC27 - GodRays (Volumetric Light Shafts)
- **Mục tiêu:** Kiểm thử vòng lặp tích lũy tia sáng tỏa tâm với 100 mẫu texture.
- **Kịch bản:** Dùng `godrays.wgsl` lấy mẫu lùi dần từ pixel về vùng sáng `[0.5, 0.2]`, áp dụng decay, density, weight và exposure.
- **Kỳ vọng:** Nền rừng rõ, có tia sáng thể tích tỏa từ vùng sáng phía trên giữa, không có ảnh đen hoặc validation error.

### TC28 - Ripple (Water/Shockwave Distortion)
- **Mục tiêu:** Kiểm thử biến dạng gợn sóng tỏa tâm với dịch chuyển UV bằng sin/cos.
- **Kịch bản:** Dùng `ripple.wgsl` lấy khoảng cách tới tâm `[0.5, 0.5]`, tạo wave theo `frequency/time/speed`, giảm biên độ theo khoảng cách và sample nền city canonical.
- **Kỳ vọng:** Nền thành phố vẫn đầy đủ, có biến dạng gợn sóng nhận diện được; không có ảnh đen hoặc validation error.

### TC29 - CRT & VHS Filter
- **Mục tiêu:** Kiểm thử barrel distortion, scanlines, vignette và RGB split trên nền sci-fi canonical.
- **Kịch bản:** Dùng `crt_vhs.wgsl` cong UV, tách RGB, tạo scanline/vignette và nhiễu integer-hash xác định.
- **Kỳ vọng:** Nền sci-fi có cong CRT, scanline/vignette/RGB split và nhiễu ổn định; không có ảnh đen hoặc validation error.

### TC30 - Dissolve & Burn Transition
- **Mục tiêu:** Kiểm thử graph hai pass gồm chroma key và dissolve/burn với noise map.
- **Kịch bản:** Tách nhân vật từ sprite sheet PNG canonical ở pass đầu, sau đó dùng noise map và viền màu cam để làm tan biến ở pass cuối.
- **Kỳ vọng:** Nền xám có các mảnh nhân vật còn lại sau dissolve và viền cháy phát sáng; không có ảnh đen, texture mất hoặc validation error.

### TC31 - Light Sweep / Shine Effect
- **Mục tiêu:** Kiểm thử graph hai pass chroma key → light sweep trên nhân vật mage.
- **Kịch bản:** Tách mage từ sprite sheet PNG canonical, sau đó tính luồng sáng xiên 45 độ bằng UV rotation và giữ nguyên alpha.
- **Kỳ vọng:** Nhân vật trên nền xám có dải sáng trắng-vàng quét chéo rõ ràng; không có ảnh đen hoặc validation error.

### TC32 - Page Curl 3D Transition
- **Mục tiêu:** Kiểm thử graph ba pass gồm hai scene độc lập và chuyển cảnh lật trang hình trụ.
- **Kịch bản:** Render scene A paladin và scene B mage với sky khác nhau, sau đó dùng dual texture page-curl ở progress 50%.
- **Kỳ vọng:** Ảnh cuối thể hiện hai scene và dải cuộn ở giữa với bóng gấp nhẹ; không có ảnh đen hoặc validation error.

### TC33 - Pixelation / Mosaic Filter
- **Mục tiêu:** Kiểm thử graph hai pass chroma key → pixelation với block size 16px.
- **Kịch bản:** Tách paladin từ sprite sheet canonical rồi snap UV theo lưới 16px trên nền xanh đậm.
- **Kỳ vọng:** Nhân vật vẫn nhận diện được với các ô mosaic vuông rõ ràng; không có ảnh đen hoặc validation error.

### TC34 - Directional Motion Blur
- **Mục tiêu:** Kiểm thử graph hai pass chroma key → directional blur 30 độ với 20 mẫu.
- **Kịch bản:** Tách mage từ sprite sheet canonical rồi tích lũy các mẫu texture dọc theo vector góc 30 độ.
- **Kỳ vọng:** Nhân vật có vệt nhòe kéo chéo rõ ràng trên nền đỏ tối; không có ảnh đen hoặc validation error.

### TC35 - Halftone / Comic Filter
- **Mục tiêu:** Kiểm thử graph hai pass chroma key → halftone với lưới điểm xoay 45 độ.
- **Kịch bản:** Tách nhân vật paladin canonical rồi chuyển vùng sáng/tối thành các chấm halftone đen/trắng theo độ sáng.
- **Kỳ vọng:** Nền vàng comic có nhân vật vẫn nhận diện được dưới dạng halftone; không có ảnh đen hoặc validation error.
- **Kết quả parity:** Desktop/Web dùng chung manifest fingerprint `0bfdc815933931d8`; vision đạt; raw khác 6 byte ở 2 pixel, sai số tối đa `1/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc35_halftone_report.md`](reports/tc35_halftone_report.md)
