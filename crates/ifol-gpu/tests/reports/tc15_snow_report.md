# Báo cáo: TC15 - Hệ hạt tuyết instancing với chuyển động vật lý

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc15_snow.json`
- **Graph fingerprint (FNV-1a):** `6ec7f347092fd77a`
- **Mô tả test case:** Render cảnh đêm tuyết xác định với sky procedural, trăng, mây, cây, nhân vật và 200 hạt tuyết instancing.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `cloud_depth.wgsl`, `moon_surface.wgsl`, `sky_composite.wgsl`, `snow_physics_instanced.wgsl`
- **Asset/input:** `canonical_bg_forest_props1.png`, `canonical_particle_snow.png`, `canonical_sprites_heroes.png`, `canonical_tc085_noise.png`, `canonical_tc085_props.png`
- **Chính sách input:** Dùng PNG canonical để Desktop/WebGPU giải mã cùng một input byte-level.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** snow_scene (Winter snow scene, target final)
- **Số pass:** `1`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `winter_sky → winter_moon → winter_cloud → pine_left → pine_right → paladin → snow_particles`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `snow_scene`
- **Graph resources:** nodes=`1`, draw commands=`7`, snow particle instances=`200`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `5.8176 ms`
- **Thời gian render lần hai (warm/cache):** `1.9775 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `66.0%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `1 pass (winter snow scene) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/desktop/tc15_snow_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `090ff1fe91e9c68e`
- **SHA-256:** `fd21d34fa66e57d2560d191d3edc8b827bd02c513a3ceb6b1a32cac58aa1e7ec`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc15_snow.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận cảnh tuyết mùa đông đúng mô tả: bầu trời, mặt trăng, mây, hai cây thông, paladin và tuyết rơi; không có khung đen hay artefact rõ ràng.
- **Graph thực tế:** nodes=1, draw commands=7, instances=200



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `311.4000 ms`
- **Thời gian render lần hai (warm/cache):** `3.1000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `99.0%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `1 pass (winter snow scene) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/web/tc15_snow_web.bin`
- **Dấu vân tay raw (FNV-1a):** `77230c8b5c42d660`
- **SHA-256:** `8ae2308471e999aaa44a38a1088847d64ba56a11e82b89f502469194b0f03ed5`
- **Ảnh:** ![WebGPU output](../outputs/web/tc15_snow_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận cảnh tuyết mùa đông đúng mô tả: bầu trời, mặt trăng, mây, hai cây thông, paladin và tuyết rơi; không có khung đen hay artefact rõ ràng.
- **Graph thực tế:** nodes=1, draw commands=7, instances=200



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `73` |
| Số pixel khác nhau | `28` |
| Sai số kênh màu lớn nhất | `24/255` |
| Khác biệt màu/presentation | `CÓ - cần theo dõi để đạt byte parity` |
| Số pixel non-background Desktop/Web | `KHÔNG ÁP DỤNG` |
| Bounding box Desktop | `KHÔNG ÁP DỤNG` |
| Bounding box WebGPU | `KHÔNG ÁP DỤNG` |
| Bounding box non-background giống nhau | `ĐẠT` |
| Số pixel mask khác nhau | `0` (ngưỡng `0`) |
| Parity cấu trúc không phụ thuộc màu | `ĐẠT` |
| Cache giữ nguyên output cold/warm ở cả hai môi trường | `ĐẠT` |
| Validation/fallback contract không panic | `ĐẠT` |
| Đúng mô tả test case | `ĐẠT` |

**Kết luận:** `ĐẠT CÓ ĐIỀU KIỆN - graph và cấu trúc render giống; khác biệt còn lại thuộc pixel/màu và nằm trong ngưỡng đã khai báo.`

## 5. Phân tích hiệu suất

Các giá trị trên đo thời gian thực thi graph, submit lệnh và chờ GPU hoàn tất;
không bao gồm khởi tạo device/pipeline hoặc readback. Vì vậy `cold` ở đây là
lần execute đầu sau khi resource/pipeline đã được tạo, không phải cold start
của toàn bộ ứng dụng. Giá trị dưới `1 ms` tương đương microsecond và cần được
đọc theo đơn vị đó khi phân tích.

## 6. Phân tích sai khác raw theo tọa độ

Raw Desktop/Web có cùng kích thước `1.920.000` byte. Có `73` byte thuộc
`28` pixel khác nhau; alpha không khác nhau và sai số lớn nhất là `24/255`.
Các sai khác được gom theo pixel để phân biệt sai khác lượng tử hóa nhỏ với
sai khác hình học:

```text
(231,198) B 56→57;
(69,347) RGB (90,92,95)→(106,109,112);
(72,350) G 248→247;
(475,389) G 194→195;
(473,390) RB (51,122)→(52,123);
(475,390) RGB (183,191,203)→(185,193,204);
(472,393) RGB (188,196,207)→(197,204,214);
(476,393) RGB (200,207,217)→(202,208,218);
(474,394) RB (136,168)→(135,167);
(477,394) RGB (104,121,147)→(101,118,145);
(475,395) RGB (128,141,162)→(129,142,163);
(477,395) RGB (188,196,207)→(186,194,205);
(481,398) RGB (82,104,136)→(85,106,137);
(363,495) RB (217,229)→(216,228);
(365,495) RGB (228,220,240)→(224,215,237);
(362,496) RGB (206,178,224)→(205,174,223);
(371,496) RB (166,192)→(167,193);
(374,497) RGB (205,212,222)→(201,208,219);
(356,498) RGB (192,164,215)→(191,161,214);
(368,498) RGB (202,209,219)→(204,211,221);
(375,498) RGB (161,171,188)→(176,185,199);
(359,499) GB (182,221)→(180,220);
(373,499) RGB (141,155,175)→(139,153,174);
(361,500) RGB (209,185,225)→(208,184,224);
(368,500) RGB (194,202,213)→(176,186,200);
(372,500) RGB (135,149,171)→(128,143,166);
(376,500) RGB (135,150,171)→(139,153,174);
(369,503) RGB (122,139,163)→(146,159,178)
```

Các điểm khác nhau tập trung ở biên/chi tiết sprite và hạt tuyết, không tạo
ra sai khác mask hoặc thay đổi bố cục. Vì vậy kết quả hiện tại đạt parity về
graph, cấu trúc và nội dung thị giác; byte parity tuyệt đối vẫn cần canonical
shader/texture sampling và quy tắc số học thống nhất ở tầng render/export.

Web có cold time `311.4 ms` và warm time `3.1 ms`; chênh lệch lớn chủ yếu là
chi phí khởi tạo/lazy compilation của lần submit đầu, không phải thay đổi
graph. Cần theo dõi riêng trong benchmark ổn định trước khi kết luận có hồi
quy hiệu suất.
