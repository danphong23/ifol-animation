# Báo cáo: TC18 - Hiệu ứng chuyển cảnh video Glitch

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc18_transition.json`
- **Graph fingerprint (FNV-1a):** `9c9b047f0733fa82`
- **Mô tả test case:** Render hai cảnh canonical rồi ghép bằng hiệu ứng chuyển cảnh glitch dual-texture xác định ở progress 50%.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `sky_composite.wgsl`, `transition.wgsl`
- **Asset/input:** `canonical_sprites_heroes.png`, `canonical_tc085_noise.png`
- **Chính sách input:** Dùng PNG canonical để Desktop/WebGPU giải mã cùng một input byte-level.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** scene_a (Purple scene A, target scene_a) → scene_b (Blue scene B, target scene_b) → transition (Glitch transition A to B, target final)
- **Số pass:** `3`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `sky_a → paladin_a → sky_b → mage_b → glitch_transition`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `scene_a → scene_b → transition`
- **Graph resources:** nodes=`3`, draw commands=`5`, tổng instances=`5`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `10.2346 ms`
- **Thời gian render lần hai (warm/cache):** `4.1496 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `59.5%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `3 pass (scene A → scene B → dual-texture transition) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/desktop/tc18_transition_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `2ebfe9ea3e7018f3`
- **SHA-256:** `26e538cfe6b6347dea09a49a27f99fc565f271dc76fe0c717fd00dcbcc551c6a`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc18_transition.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận ảnh chuyển cảnh glitch ở progress 50%, gồm block shift ngang, RGB split rõ, chuyển giữa cảnh tím có paladin và cảnh xanh có mage; không có black output hoặc artefact ngoài hiệu ứng được mô tả.
- **Graph thực tế:** nodes=3, draw commands=5, instances=5



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `12.1000 ms`
- **Thời gian render lần hai (warm/cache):** `8.4000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `30.6%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `3 pass (scene A → scene B → dual-texture transition) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/web/tc18_transition_web.bin`
- **Dấu vân tay raw (FNV-1a):** `33aa21d26ba123d6`
- **SHA-256:** `821ee5dfe9694114328ca5a03f4331be29bb7b6cf440dc083ec0b410a6d80ab5`
- **Ảnh:** ![WebGPU output](../outputs/web/tc18_transition_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận ảnh chuyển cảnh glitch ở progress 50%, gồm block shift ngang, RGB split rõ, chuyển giữa cảnh tím có paladin và cảnh xanh có mage; không có black output hoặc artefact ngoài hiệu ứng được mô tả.
- **Graph thực tế:** nodes=3, draw commands=5, instances=5



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `1` |
| Số pixel khác nhau | `1` |
| Sai số kênh màu lớn nhất | `1/255` |
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
