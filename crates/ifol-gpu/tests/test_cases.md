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

### TC19 - Audio-Reactive Spectrum Visualizer
- **Mục tiêu:** Kiểm thử truyền mảng 16 dải tần qua uniform buffer và dựng phổ âm thanh neon xác định.
- **Kịch bản:** Shader nhận 16 frequency bands được đóng gói thành bốn `vec4`, màu cyan cơ sở, thời gian cố định và texture noise canonical để tạo nền grid, cột phổ, glow và peak line.
- **Kỳ vọng hình ảnh xuất ra:** Nền grid neon cyan/tím với 16 cột có chiều cao khác nhau, glow và peak line rõ; không có black output hoặc validation error.

---

## Nhóm 4: Bài Thi Masterpiece Tích Hợp (TC20)

### TC20 - 3D Perspective Projection & Card Flip (2.5D)
- **Mục tiêu:** Kiểm thử truyền ma trận model-view-projection cố định vào WGSL và render sprite 2D trong phối cảnh 3D.
- **Kịch bản:** Một sprite paladin canonical được crop, xoay theo MVP với phối cảnh camera, đồng thời áp dụng chroma-key/despill.
- **Kỳ vọng hình ảnh xuất ra:** Sprite nghiêng theo phối cảnh trên nền xám, crop và alpha đúng, không có black output hoặc validation error.

### TC21 - SDF Masking & Chroma Key
- **Mục tiêu:** Kiểm thử mask tròn SDF kết hợp chroma-key/despill cho avatar.
- **Kịch bản:** Crop avatar paladin canonical, áp dụng transform cố định, lọc phông xanh rồi nhân alpha với mask tròn mềm trong local space.
- **Kỳ vọng hình ảnh xuất ra:** Avatar nằm trong vùng mask tròn, nền xám bên ngoài được giữ nguyên, không có black output hoặc validation error.

### TC22 - Hardware Instancing (Props)
- **Mục tiêu:** Kiểm thử 100 instance phần cứng bằng một draw command.
- **Kịch bản:** Dùng chung một crop sprite canonical, shader tạo vị trí, scale và rotation xác định theo `instance_index`, có sửa aspect ratio và loại phông xanh.
- **Kỳ vọng hình ảnh xuất ra:** 100 prop nhỏ phân bố xác định trên nền xanh đậm, không có black output hoặc validation error.

### TC23 - Palette Swap (HSV Shift)
- **Mục tiêu:** Kiểm thử thay bảng màu HSV từ hồng sang cyan nhưng giữ shading/highlight.
- **Kịch bản:** Crop sprite canonical, dùng `color_replace.wgsl` để tính khoảng cách HSV, dịch hue/saturation, giữ value và loại phông xanh.
- **Kỳ vọng hình ảnh xuất ra:** Giáp và chi tiết chuyển sang cyan, shading/highlight giữ nguyên, không có output đen hoặc lỗi validation.

### TC24 - Vertex Deformation (Wind/Sway)
- **Mục tiêu:** Kiểm thử biến dạng đỉnh của sprite theo hiệu ứng gió xác định.
- **Kịch bản:** Dùng `distortion.wgsl` để neo phần dưới sprite và uốn phần trên theo `sin(time * frequency) * amplitude`.
- **Kỳ vọng hình ảnh xuất ra:** Sprite nằm trên nền xám, phần dưới được neo, phần trên uốn theo gió; không có black output hoặc lỗi validation.

### TC25 - Fake Rim Lighting & Drop Shadow
- **Mục tiêu:** Kiểm thử rim light và drop shadow bằng hai instance trong một draw command.
- **Kịch bản:** Instance 0 dịch sprite để tạo bóng đen bán trong suốt; instance 1 render sprite chính và tính viền sáng vàng bằng cách dò alpha lân cận.
- **Kỳ vọng hình ảnh xuất ra:** Sprite chính có viền sáng vàng và bóng đổ lệch phía sau, không có black output hoặc lỗi validation.

### TC26 - Glitch & Chromatic Aberration
- **Mục tiêu:** Kiểm thử glitch theo dải ngang và tách kênh RGB với hash xác định giữa backend.
- **Kịch bản:** Dùng `glitch.wgsl` tạo block shift theo integer hash từ `time`, sau đó lấy mẫu lệch cho kênh đỏ/xanh và loại phông xanh.
- **Kỳ vọng hình ảnh xuất ra:** Sprite vẫn nhận diện được nhưng có dải glitch ngang và RGB split rõ, không có black output hoặc lỗi validation.

### TC27 - GodRays (Volumetric Light Shafts)
- **Mục tiêu:** Kiểm thử vòng lặp tích lũy tia sáng tỏa tâm với 100 mẫu texture.
- **Kịch bản:** Dùng `godrays.wgsl` lấy mẫu lùi dần từ pixel về vùng sáng `[0.5, 0.2]`, áp dụng decay, density, weight và exposure.
- **Kỳ vọng hình ảnh xuất ra:** Nền rừng rõ, có tia sáng thể tích tỏa từ vùng sáng phía trên giữa, không có ảnh đen hoặc lỗi validation.

### TC28 - Ripple (Water/Shockwave Distortion)
- **Mục tiêu:** Kiểm thử biến dạng gợn sóng tỏa tâm với dịch chuyển UV bằng sin/cos.
- **Kịch bản:** Dùng `ripple.wgsl` lấy khoảng cách tới tâm `[0.5, 0.5]`, tạo wave theo `frequency/time/speed`, giảm biên độ theo khoảng cách và sample nền city canonical.
- **Kỳ vọng hình ảnh xuất ra:** Nền thành phố vẫn đầy đủ, có biến dạng gợn sóng nhận diện được; không có ảnh đen hoặc lỗi validation.

### TC29 - CRT & VHS Filter
- **Mục tiêu:** Kiểm thử barrel distortion, scanlines, vignette và RGB split trên nền sci-fi canonical.
- **Kịch bản:** Dùng `crt_vhs.wgsl` cong UV, tách RGB, tạo scanline/vignette và nhiễu integer-hash xác định.
- **Kỳ vọng hình ảnh xuất ra:** Nền sci-fi có cong CRT, scanline/vignette/RGB split và nhiễu ổn định; không có ảnh đen hoặc lỗi validation.

### TC30 - Dissolve & Burn Transition
- **Mục tiêu:** Kiểm thử graph hai pass gồm chroma key và dissolve/burn với noise map.
- **Kịch bản:** Tách nhân vật từ sprite sheet PNG canonical ở pass đầu, sau đó dùng noise map và viền màu cam để làm tan biến ở pass cuối.
- **Kỳ vọng hình ảnh xuất ra:** Nền xám có các mảnh nhân vật còn lại sau dissolve và viền cháy phát sáng; không có ảnh đen, texture mất hoặc lỗi validation.

### TC31 - Light Sweep / Shine Effect
- **Mục tiêu:** Kiểm thử graph hai pass chroma key → light sweep trên nhân vật mage.
- **Kịch bản:** Tách mage từ sprite sheet PNG canonical, sau đó tính luồng sáng xiên 45 độ bằng UV rotation và giữ nguyên alpha.
- **Kỳ vọng hình ảnh xuất ra:** Nhân vật trên nền xám có dải sáng trắng-vàng quét chéo rõ ràng; không có ảnh đen hoặc lỗi validation.

### TC32 - Page Curl 3D Transition
- **Mục tiêu:** Kiểm thử graph ba pass gồm hai scene độc lập và chuyển cảnh lật trang hình trụ.
- **Kịch bản:** Render scene A paladin và scene B mage với sky khác nhau, sau đó dùng dual texture page-curl ở progress 50%.
- **Kỳ vọng hình ảnh xuất ra:** Ảnh cuối thể hiện hai scene và dải cuộn ở giữa với bóng gấp nhẹ; không có ảnh đen hoặc lỗi validation.

### TC33 - Pixelation / Mosaic Filter
- **Mục tiêu:** Kiểm thử graph hai pass chroma key → pixelation với block size 16px.
- **Kịch bản:** Tách paladin từ sprite sheet canonical rồi snap UV theo lưới 16px trên nền xanh đậm.
- **Kỳ vọng hình ảnh xuất ra:** Nhân vật vẫn nhận diện được với các ô mosaic vuông rõ ràng; không có ảnh đen hoặc lỗi validation.

### TC34 - Directional Motion Blur
- **Mục tiêu:** Kiểm thử graph hai pass chroma key → directional blur 30 độ với 20 mẫu.
- **Kịch bản:** Tách mage từ sprite sheet canonical rồi tích lũy các mẫu texture dọc theo vector góc 30 độ.
- **Kỳ vọng hình ảnh xuất ra:** Nhân vật có vệt nhòe kéo chéo rõ ràng trên nền đỏ tối; không có ảnh đen hoặc lỗi validation.

### TC35 - Halftone / Comic Filter
- **Mục tiêu:** Kiểm thử graph hai pass chroma key → halftone với lưới điểm xoay 45 độ.
- **Kịch bản:** Tách nhân vật paladin canonical rồi chuyển vùng sáng/tối thành các chấm halftone đen/trắng theo độ sáng.
- **Kỳ vọng hình ảnh xuất ra:** Nền vàng comic có nhân vật vẫn nhận diện được dưới dạng halftone; không có ảnh đen hoặc lỗi validation.
- **Kết quả parity:** Desktop/Web dùng chung manifest fingerprint `0bfdc815933931d8`; vision đạt; raw khác 6 byte ở 2 pixel, sai số tối đa `1/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc35_halftone_report.md`](reports/tc35_halftone_report.md)

### TC36 - Radial Blur / Zoom Blur
- **Mục tiêu:** Kiểm thử graph hai pass chroma key → radial blur với 30 mẫu và trọng số giảm dần.
- **Kịch bản:** Tách nhân vật paladin canonical rồi lấy mẫu dọc hướng từ pixel về tâm `[0.5, 0.5]` với strength `0.15`.
- **Kỳ vọng hình ảnh xuất ra:** Nền tím tối có nhân vật bị kéo mờ tỏa tâm nhưng vẫn nhận diện được; không có ảnh đen hoặc lỗi validation.
- **Kết quả parity:** Desktop/Web dùng chung manifest fingerprint `e8635023d0c9c2fb`; vision đạt; raw parity tuyệt đối `0 byte` khác.
- **Báo cáo:** [`tc36_radial_blur_report.md`](reports/tc36_radial_blur_report.md)

### TC37 - Chromatic Aberration / RGB Split
- **Mục tiêu:** Kiểm thử graph hai pass chroma key → phân tách kênh RGB theo khoảng cách tới tâm.
- **Kịch bản:** Tách paladin canonical rồi lấy mẫu riêng R/G/B với độ lệch xuyên tâm `amount=0.1`.
- **Kỳ vọng hình ảnh xuất ra:** Viền đỏ/xanh tách nhẹ quanh paladin trên nền xanh ngọc; không có ảnh đen hoặc lỗi validation.
- **Kết quả parity:** Desktop/Web dùng chung manifest fingerprint `7f5f010b70f54583`; vision đạt; raw parity tuyệt đối `0 byte` khác.
- **Báo cáo:** [`tc37_chromatic_aberration_report.md`](reports/tc37_chromatic_aberration_report.md)

### TC38 - Kaleidoscope
- **Mục tiêu:** Kiểm thử graph hai pass chroma key → gập tọa độ cực thành sáu phân đoạn đối xứng.
- **Kịch bản:** Tách mage canonical rồi áp dụng polar mapping, modulo và angular fold với `segments=6`.
- **Kỳ vọng hình ảnh xuất ra:** Họa tiết kính vạn hoa sáu nhánh trên nền tím; không có ảnh đen hoặc lỗi validation.
- **Kết quả parity:** Desktop/Web dùng chung manifest fingerprint `cf4713957e83abbf`; vision đạt; raw khác 63 byte ở 47 pixel, tối đa `1/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc38_kaleidoscope_report.md`](reports/tc38_kaleidoscope_report.md)

### TC39 - Hologram Scanlines
- **Mục tiêu:** Kiểm thử graph hai pass chroma key → sọc quét hologram cyan xác định.
- **Kịch bản:** Tách mage canonical rồi điều chế màu/alpha bằng sóng sin với 200 dòng, time cố định `1.0`.
- **Kỳ vọng hình ảnh xuất ra:** Mage có sọc quét ngang cyan và hiệu ứng hologram; không có ảnh đen hoặc lỗi validation.
- **Kết quả parity:** Desktop/Web dùng chung manifest fingerprint `5ea108ce90344f78`; vision đạt; raw khác 5 byte ở 5 pixel, tối đa `1/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc39_scanlines_report.md`](reports/tc39_scanlines_report.md)

### TC40 - Vignette và hạt phim
- **Mục tiêu:** Kiểm thử graph chroma key → vignette/grain deterministic.
- **Kỳ vọng hình ảnh xuất ra:** Mage giữ đúng bố cục, tối viền và có hạt phim ổn định.
- **Kết quả parity:** Desktop/Web dùng fingerprint `da19870721a1d0ee`; vision đạt; raw khác 36605 byte ở 12345 pixel, sai số tối đa `147/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc40_vignette_grain_report.md`](reports/tc40_vignette_grain_report.md)

### TC41 - Aspect Fill
- **Mục tiêu:** Kiểm thử fill ảnh theo tỷ lệ đích với nền blur bảo toàn bố cục.
- **Kỳ vọng hình ảnh xuất ra:** Ảnh Sci-Fi fill khung 9:16, foreground không méo, nền được blur.
- **Kết quả parity:** Desktop/Web dùng fingerprint `039c82c31366c5f1`; vision đạt; raw khác 13 byte ở 13 pixel, sai số tối đa `1/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc41_aspect_fill_report.md`](reports/tc41_aspect_fill_report.md)

### TC42 - HDR Bloom
- **Mục tiêu:** Kiểm thử graph chroma key → bloom → composite với vùng sáng emissive.
- **Kỳ vọng hình ảnh xuất ra:** Mage trên nền Sci-Fi có bloom lan rộng, không bị cắt vùng sáng.
- **Kết quả parity:** Desktop/Web dùng fingerprint `ded6885c267c0133`; vision đạt; raw khác 1 byte ở 1 pixel, sai số tối đa `1/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc42_hdr_bloom_report.md`](reports/tc42_hdr_bloom_report.md)

### TC43 - Track Matte
- **Mục tiêu:** Kiểm thử alpha track matte giới hạn texture nền theo silhouette nhân vật.
- **Kỳ vọng hình ảnh xuất ra:** Sci-Fi chỉ hiện bên trong silhouette paladin.
- **Kết quả parity:** Desktop/Web dùng fingerprint `3824afc9c439d9b6`; vision đạt; raw parity tuyệt đối `0 byte` khác.
- **Báo cáo:** [`tc43_track_matte_report.md`](reports/tc43_track_matte_report.md)

### TC44 - Anamorphic Flare
- **Mục tiêu:** Kiểm thử flare ngang từ vùng sáng bằng shader dùng chung.
- **Kỳ vọng hình ảnh xuất ra:** Có streak xanh lam ngang trên cảnh Sci-Fi, không có black output.
- **Kết quả parity:** Desktop/Web dùng fingerprint `8acb587266daa9a5`; vision đạt; raw khác 113 byte ở 106 pixel, sai số tối đa `77/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc44_anamorphic_flare_report.md`](reports/tc44_anamorphic_flare_report.md)

### TC45 - Glassmorphism
- **Mục tiêu:** Kiểm thử panel kính mờ bo góc, blur/refraction và rim light.
- **Kỳ vọng hình ảnh xuất ra:** Paladin và panel kính cùng xuất hiện đúng thứ tự layer.
- **Kết quả parity:** Desktop/Web dùng fingerprint `2d3810a87e84d9ac`; vision đạt; raw khác 1 byte ở 1 pixel, sai số tối đa `1/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc45_glassmorphism_report.md`](reports/tc45_glassmorphism_report.md)

### TC46 - Selective Color
- **Mục tiêu:** Kiểm thử chuyển grayscale có chọn lọc theo hue mục tiêu.
- **Kỳ vọng hình ảnh xuất ra:** Toàn cảnh giảm bão hòa, vùng màu mục tiêu vẫn giữ màu.
- **Kết quả parity:** Desktop/Web dùng fingerprint `34ecb2f54ddfd44c`; vision đạt; raw parity tuyệt đối `0 byte` khác.
- **Báo cáo:** [`tc46_selective_color_report.md`](reports/tc46_selective_color_report.md)

### TC47 - Motion Echo
- **Mục tiêu:** Kiểm thử nhiều echo theo velocity với alpha và màu giảm dần.
- **Kỳ vọng hình ảnh xuất ra:** Mage có các bóng chuyển động lệch nhau trên nền, không lỗi validation.
- **Kết quả parity:** Desktop/Web dùng fingerprint `170a3d531712c956`; vision đạt; raw parity tuyệt đối `0 byte` khác.
- **Báo cáo:** [`tc47_motion_echo_report.md`](reports/tc47_motion_echo_report.md)

### TC48 - Bokeh Depth of Field
- **Mục tiêu:** Kiểm thử blur theo vùng focus và bokeh highlight.
- **Kỳ vọng hình ảnh xuất ra:** Foreground rõ, nền blur/bokeh và highlight được tăng cường.
- **Kết quả parity:** Desktop/Web dùng fingerprint `6fefabed9d0cf1d5`; vision đạt; raw khác 71 byte ở 71 pixel, sai số tối đa `1/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc48_bokeh_dof_report.md`](reports/tc48_bokeh_dof_report.md)

### TC49 - Trim Paths
- **Mục tiêu:** Kiểm thử stroke bo góc nét đứt bị trim theo khoảng start/end.
- **Kỳ vọng hình ảnh xuất ra:** Khung cyan quanh mage chỉ hiển thị đoạn stroke được trim.
- **Kết quả parity:** Desktop/Web dùng fingerprint `49cdb72f893223c4`; vision đạt; raw parity tuyệt đối `0 byte` khác.
- **Báo cáo:** [`tc49_trim_paths_report.md`](reports/tc49_trim_paths_report.md)

### TC50 - Exposure Inspector
- **Mục tiêu:** Kiểm thử overlay zebra và false-color IRE trên scene canonical.
- **Kỳ vọng hình ảnh xuất ra:** Nửa trái hiển thị zebra vùng cháy sáng, nửa phải hiển thị false-color IRE và vạch chia trắng.
- **Kết quả parity:** Desktop/Web dùng fingerprint `6eb21c3021072252`; vision đạt; raw khác 96 byte ở 32 pixel, sai số tối đa `166/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc50_exposure_inspector_report.md`](reports/tc50_exposure_inspector_report.md)

### TC51 - Texture Atlas Bleed Prevention
- **Mục tiêu:** Kiểm thử kẹp biên nửa texel cho hai sprite liền kề trong atlas.
- **Kỳ vọng hình ảnh xuất ra:** Paladin và mage không bị lem màu từ ô atlas bên cạnh.
- **Kết quả parity:** Desktop/Web dùng fingerprint `d8d597349c97b340`; vision đạt sau khi sửa ABI `key_color` của Web; raw khác 163 byte ở 128 pixel, sai số tối đa `3/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc51_atlas_clamp_report.md`](reports/tc51_atlas_clamp_report.md)

### TC52 - Soft Particle Depth Fading
- **Mục tiêu:** Kiểm thử depth test và additive volumetric energy sphere.
- **Kỳ vọng hình ảnh xuất ra:** Quả cầu plasma cyan giao thoa mềm với paladin, không có hard intersection.
- **Kết quả parity:** Desktop/Web dùng fingerprint `3f930de62616d52f`; vision đạt; raw khác 4495 byte ở 1554 pixel, sai số tối đa `101/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc52_soft_particles_report.md`](reports/tc52_soft_particles_report.md)

### TC53 - Blend Modes Matrix
- **Mục tiêu:** Kiểm thử tám công thức blend deterministic trên ma trận 4x2 với nền và sprite canonical.
- **Kỳ vọng hình ảnh xuất ra:** Tám ô Normal, Multiply, Screen, Overlay, Hard Light, Soft Light, Color Dodge và Difference; có grid trắng, không mất ô.
- **Kết quả parity:** Desktop/Web dùng fingerprint `0045bf536afcf57d`; vision và cấu trúc đạt; raw khác 69 byte ở 51 pixel, sai số tối đa `1/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc53_blend_modes_report.md`](reports/tc53_blend_modes_report.md)

### TC54 - Indexed Flag Mesh Wave
- **Mục tiêu:** Kiểm thử mesh indexed 32x32, biến dạng đỉnh deterministic và chiếu sáng Phong.
- **Kỳ vọng hình ảnh xuất ra:** Vùng mesh cờ phủ đúng bố cục, có biến dạng sóng/lighting, không mất index hoặc validation error.
- **Kết quả parity:** Desktop/Web dùng fingerprint `99296555552df541`; vision và cấu trúc đạt; raw khác 976 byte ở 350 pixel, sai số tối đa `71/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc54_flag_mesh_report.md`](reports/tc54_flag_mesh_report.md)

### TC55 - Dual Kawase Bloom
- **Mục tiêu:** Kiểm thử chuỗi extract → downsample 400x300 → composite bloom và foreground sắc nét.
- **Kỳ vọng hình ảnh xuất ra:** Mage sắc nét trên nền Sci-Fi, bloom mềm không làm mất foreground hoặc sai tỷ lệ.
- **Kết quả parity:** Desktop/Web dùng fingerprint `2a88441e6a8ac270`; vision đạt và raw byte parity tuyệt đối `0 byte` khác.
- **Báo cáo:** [`tc55_dual_kawase_report.md`](reports/tc55_dual_kawase_report.md)

### TC56 - Dynamic Target Resizing
- **Mục tiêu:** Kiểm thử hai RenderTarget 400x600 và composition cuối 800x600.
- **Kỳ vọng hình ảnh xuất ra:** Wizard và paladin nằm ở hai panel dọc đúng tỷ lệ trên nền anime city.
- **Kết quả parity:** Desktop/Web dùng fingerprint `712b3ac12833ff81`; vision và cấu trúc đạt; raw khác 5200 byte ở 1979 pixel, sai số tối đa `37/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc56_dynamic_resize_report.md`](reports/tc56_dynamic_resize_report.md)

### TC57 - Stencil Mask Portal
- **Mục tiêu:** Kiểm thử stencil IncrementClamp/NotEqual cho portal tròn.
- **Kỳ vọng hình ảnh xuất ra:** Night-sky và wizard chỉ xuất hiện trong portal, ngoài mask giữ màu nền.
- **Kết quả parity:** Desktop/Web dùng fingerprint `33c65cd0ace1f7da`; vision và cấu trúc đạt; raw khác 836 byte ở 296 pixel, sai số tối đa `139/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc57_stencil_mask_report.md`](reports/tc57_stencil_mask_report.md)

### TC58 - MRT G-Buffer
- **Mục tiêu:** Kiểm thử ghi đồng thời albedo/emissive vào hai color attachment trong một MRT pass.
- **Kỳ vọng hình ảnh xuất ra:** Albedo bên trái và emissive mask bên phải đều có dữ liệu, sau đó composite side-by-side.
- **Kết quả parity:** Desktop/Web dùng fingerprint `99bc2711d6947215`; vision đạt và raw byte parity tuyệt đối `0 byte` khác.
- **Báo cáo:** [`tc58_mrt_gbuffer_report.md`](reports/tc58_mrt_gbuffer_report.md)

### TC59 - Sampler Address Modes
- **Mục tiêu:** Kiểm thử Repeat, MirrorRepeat và ClampToEdge với UV vượt ngoài [0,1].
- **Kỳ vọng hình ảnh xuất ra:** Ba panel cùng texture lần lượt lặp, phản chiếu và kéo dài mép.
- **Kết quả parity:** Desktop/Web dùng fingerprint `41c657787fe74841`; vision và cấu trúc đạt; raw khác 24752 byte ở 16675 pixel, sai số tối đa `3/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc59_sampler_modes_report.md`](reports/tc59_sampler_modes_report.md)

### TC60 - Ping-Pong Feedback
- **Mục tiêu:** Kiểm thử 8 chu kỳ ping-pong tạo echo trail từ wizard canonical.
- **Kỳ vọng hình ảnh xuất ra:** Wizard có các bóng mờ đồng tâm, opacity giảm dần và không phụ thuộc trạng thái target cũ.
- **Kết quả parity:** Desktop/Web dùng fingerprint `861e27bfb471246e`; vision và cấu trúc đạt; raw khác 6165 byte ở 4032 pixel, sai số tối đa `5/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc60_ping_pong_report.md`](reports/tc60_ping_pong_report.md)

### TC61 - Compute Storage Buffer Arithmetic
- **Mục tiêu:** Tính 10.240 vec4 bằng compute, đối chiếu CPU rồi render plot A/B/C.
- **Kỳ vọng hình ảnh xuất ra:** Grid có đường A vàng, B cam và C cyan; numeric readback khớp dưới `1e-4`.
- **Kết quả parity:** Desktop/Web dùng fingerprint `91a37c1c43c4f64c`; vision/validation đạt; raw khác 52 byte ở 49 pixel, sai số tối đa `1/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`. Desktop khớp CPU 10.240/10.240, max diff `0.00005054`.
- **Báo cáo:** [`tc61_compute_buffer_math_report.md`](reports/tc61_compute_buffer_math_report.md)

### TC62 - Storage Texture Sobel
- **Mục tiêu:** Kiểm thử compute ghi storage texture và Sobel edge trên ảnh sprite.
- **Kỳ vọng hình ảnh xuất ra:** Nửa trái là ảnh nhân vật gốc, divider cyan ở giữa, nửa phải là edge neon trên nền tối.
- **Kết quả parity:** Desktop/Web dùng fingerprint `29f38bc13430eb96`; validation và vision đạt; raw khác 59987 byte ở 31132 pixel, sai số tối đa `128/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc62_storage_texture_report.md`](reports/tc62_storage_texture_report.md)

### TC63 - 100k Particles
- **Mục tiêu:** Kiểm thử compute update 100.000 particle và instanced rendering deterministic.
- **Kỳ vọng hình ảnh xuất ra:** Thiên hà spiral có lõi sáng, gradient cyan/magenta và phân bố hạt dày, không rỗng hoặc có artifact cấu trúc.
- **Kết quả parity:** Desktop/Web dùng fingerprint `49c5ea09d42ea7cb`; validation và vision đạt; raw khác 204777 byte ở 74419 pixel, sai số tối đa `222/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`. Particle buffer được reset trước warm.
- **Báo cáo:** [`tc63_particles_100k_report.md`](reports/tc63_particles_100k_report.md)

### TC64 - Audio FFT Visualizer
- **Mục tiêu:** Kiểm thử compute FFT/energy bins từ PCM xác định và render visualizer.
- **Kỳ vọng hình ảnh xuất ra:** Waveform cyan ở phần trên, divider, grid và 64 cột FFT gradient ở phần dưới với peak hợp lệ.
- **Kết quả parity:** Desktop/Web dùng fingerprint `eb63136e435ed1cb`; validation và vision đạt; raw khác 3893 byte ở 2088 pixel, sai số tối đa `99/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc64_audio_fft_report.md`](reports/tc64_audio_fft_report.md)

### TC65 - Workgroup Shared Memory Blur
- **Mục tiêu:** Kiểm thử blur Gaussian 9x9 dùng tile 24x24 trong workgroup memory.
- **Kỳ vọng hình ảnh xuất ra:** Nửa trái giữ ảnh nhân vật sắc nét, divider vàng ở giữa, nửa phải blur mượt không artifact.
- **Kết quả parity:** Desktop/Web dùng fingerprint `9219b57bf1c71f6b`; validation và vision đạt; raw khác 56353 byte ở 43747 pixel, sai số tối đa `7/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc65_workgroup_blur_report.md`](reports/tc65_workgroup_blur_report.md)

### TC66 - Parallel Luminance Histogram
- **Mục tiêu:** Tính histogram 256 bin bằng atomic workgroup và vẽ overlay lên ảnh nguồn.
- **Kỳ vọng hình ảnh xuất ra:** Ảnh nguồn vẫn rõ, histogram nằm ở góc phải; tổng 256 bin phải bằng 480000 pixel.
- **Kết quả parity:** Desktop/Web dùng fingerprint `52de157767d72d36`; validation, vision và numeric readback `480000/480000` đạt; raw khác 74771 byte ở 54379 pixel, sai số tối đa `242/255`, nên `ĐẠT CÓ ĐIỀU KIỆN` do decoder/format input JPEG.
- **Báo cáo:** [`tc66_histogram_report.md`](reports/tc66_histogram_report.md)

### TC67 - Reaction Diffusion Ping-Pong
- **Mục tiêu:** Kiểm thử 2.480 bước Gray-Scott qua hai storage texture luân phiên và color mapping.
- **Kỳ vọng hình ảnh xuất ra:** Ba pattern hữu cơ cyan/hồng phát triển từ seed trên nền tím tối, không đen toàn ảnh hoặc resource hazard.
- **Kết quả parity:** Desktop/Web dùng fingerprint `92b7444c45f8deee`; validation và vision đạt; raw khác 8095 byte ở 6380 pixel, sai số tối đa `10/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`. Seed được reset trước warm.
- **Báo cáo:** [`tc67_pingpong_report.md`](reports/tc67_pingpong_report.md)

### TC68 - Verlet Chain Simulation
- **Mục tiêu:** Tích phân và giải ràng buộc 256 chuỗi, mỗi chuỗi 16 node, trong 100 bước rồi render 4.096 node instanced.
- **Kỳ vọng hình ảnh xuất ra:** Các chuỗi node tím-cyan ổn định trên nền xám, không nổ/NaN và không có vùng rác.
- **Kết quả parity:** Desktop/Web dùng fingerprint `57c2a130c0067d22`; validation, numeric finite-node `4096/4096`, vision và cold/warm đều đạt. Raw khác 48 byte ở 17 pixel, sai số tối đa `166/255`, nên `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc68_verlet_report.md`](reports/tc68_verlet_report.md)

### TC69 - Compute-Driven Vertex Deformation
- **Mục tiêu:** Biến dạng lưới indexed 65x65 bằng compute và dùng trực tiếp dest storage buffer làm vertex buffer zero-copy.
- **Kỳ vọng hình ảnh xuất ra:** Lưới 4.225 vertex/24.576 index có twist/ripple và màu biến dạng, không mất index.
- **Kết quả parity:** Desktop/Web dùng fingerprint `940d2398c4b39343`; validation, vision và cold/warm đều đạt. Raw khác 1.501 byte ở 1.297 pixel, sai số tối đa `247/255`, nhưng parity cấu trúc đạt; phân loại `ĐẠT CÓ ĐIỀU KIỆN`.
- **Báo cáo:** [`tc69_deformation_report.md`](reports/tc69_deformation_report.md)

### TC70 - GPU Particle Culling and Indirect Draw
- **Mục tiêu:** Culling 100.000 particle vào compact buffer rồi dùng GPU-written instance count cho indirect draw.
- **Kỳ vọng hình ảnh xuất ra:** Chỉ còn các hạt xanh trong vùng culling trung tâm; ngoài vùng rỗng và không có hạt rác.
- **Kết quả parity:** Desktop/Web dùng fingerprint `208bace8904bea29`; validation, indirect count, vision và cold/warm đều đạt. Raw parity tuyệt đối: `0` byte và `0` pixel khác; phân loại `ĐẠT`.
- **Báo cáo:** [`tc70_culling_report.md`](reports/tc70_culling_report.md)

## Trạng thái chứng nhận sau TC70

TC01–TC70 là phạm vi bằng chứng hiện tại; TC68–TC70 là batch đã commit gần
nhất. TC71–TC73 chưa được tính là pass chính thức: TC71 còn nondeterministic ở
shader/graph test contract, còn TC72–TC73 chưa hoàn tất bằng chứng preview/report.
Không mở rộng TC74+ trước khi baseline và ranh giới core/test harness/tầng media
được chốt. Tiêu chí chi tiết nằm tại
[`90-validation-boundary-and-clean-baseline.md`](../docs/70-status/90-validation-boundary-and-clean-baseline.md).
