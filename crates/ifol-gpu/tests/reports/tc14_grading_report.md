# Báo cáo: TC14 - Color grading điện ảnh và ACES Filmic tone mapping

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc14_grading.json`
- **Graph fingerprint (FNV-1a):** `f3824201007dd4a7`
- **Mô tả test case:** Render cảnh hoàng hôn canonical, sau đó áp dụng color grading filmic, split toning và vignette xác định trong pass thứ hai.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `color_grading_filmic.wgsl`, `sky_composite.wgsl`, `star_particles_sprite.wgsl`
- **Asset/input:** `canonical_bg_forest_props1.png`, `canonical_sprites_heroes.png`, `canonical_sprites_items.png`, `canonical_tc085_noise.png`, `canonical_tc085_props.png`
- **Chính sách input:** Dùng PNG canonical để Desktop/WebGPU giải mã cùng một input byte-level.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** scene_pass (Sunset scene, target scene) → grading_pass (ACES filmic color grading, target final)
- **Số pass:** `2`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `sunset_sky → sunset_sparks → tree_left → tree_right → paladin → mage → chest → color_grade`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `scene_pass → grading_pass`
- **Graph resources:** nodes=`2`, draw commands=`8`, tổng instances=`47`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `7.2701 ms`
- **Thời gian render lần hai (warm/cache):** `2.3744 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `67.3%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `2 pass (scene → color grading) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/desktop/tc14_grading_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `98470a5cb883d16f`
- **SHA-256:** `0ceab230bfdd82a04b7d88e2a67ec909b69de05791048627f2d520b165dbd156`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc14_grading.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Cảnh hoàng hôn có gradient hồng-vàng, shadow pha tím chàm, highlight ấm, vignette mềm; cây, nhân vật và chest còn chi tiết, không có artifact rõ ràng.
- **Graph thực tế:** nodes=2, draw commands=8, instances=None



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `9.5000 ms`
- **Thời gian render lần hai (warm/cache):** `6.7000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `29.5%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `2 pass (scene → color grading) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/web/tc14_grading_web.bin`
- **Dấu vân tay raw (FNV-1a):** `210331896ad2a4d5`
- **SHA-256:** `ee444d89335fc7f88a81ac4d880421a8147381bd562f19f9411abfb4d2453bd5`
- **Ảnh:** ![WebGPU output](../outputs/web/tc14_grading_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Cảnh Web có cùng bố cục, tông màu grading, split-tone và vignette; cây, nhân vật và chest còn chi tiết, không có artifact rõ ràng.
- **Graph thực tế:** nodes=2, draw commands=8, instances=None



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `18` |
| Số pixel khác nhau | `16` |
| Sai số kênh màu lớn nhất | `2/255` |
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

### Chi tiết raw diff

18 byte khác nhau nằm tại 16 pixel; alpha không khác. Tọa độ dùng `(x, y)` và
giá trị là `Desktop → WebGPU` trên thang `0..255`:

```text
(73,19) G 37→36; (736,41) R 145→146; (374,45) B 137→136;
(343,95) B 146→145; (217,149) B 2→1; (666,220) B 95→94;
(710,237) B 88→89; (462,287) G 124→125; (344,402) R 59→60;
(293,421) B 99→98; (544,460) R 188→189; (353,465) B 3→4;
(274,475) R 189→190; (310,577) G 120→119, B 27→26;
(630,577) G 115→113, B 26→24; (55,594) B 17→18.
```

Sai khác chỉ là sai số lượng tử hóa rất nhỏ sau color grading/ACES ở các pixel
biên hoặc vùng chuyển sắc; vision và cấu trúc không bị ảnh hưởng. Đây vẫn là
điểm cần xử lý nếu mục tiêu cuối cùng là byte-exact cho mọi graph.

## 5. Phân tích hiệu suất

Các giá trị trên đo thời gian thực thi graph, submit lệnh và chờ GPU hoàn tất;
không bao gồm khởi tạo device/pipeline hoặc readback. Vì vậy `cold` ở đây là
lần execute đầu sau khi resource/pipeline đã được tạo, không phải cold start
của toàn bộ ứng dụng. Giá trị dưới `1 ms` tương đương microsecond và cần được
đọc theo đơn vị đó khi phân tích.
