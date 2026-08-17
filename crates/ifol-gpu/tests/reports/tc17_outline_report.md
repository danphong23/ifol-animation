# Báo cáo: TC17 - Outline nhiều pass và bóng đổ hậu kỳ

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc17_outline.json`
- **Graph fingerprint (FNV-1a):** `bd209137e1b026dc`
- **Mô tả test case:** Render sprite canonical vào layer offscreen trong suốt, sau đó ghép với sky bằng outline và drop shadow lấy từ alpha.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `outline_shadow.wgsl`, `sky_composite.wgsl`
- **Asset/input:** `canonical_sprites_heroes.png`, `canonical_sprites_items.png`, `canonical_tc085_noise.png`
- **Chính sách input:** Dùng PNG canonical để Desktop/WebGPU giải mã cùng một input byte-level.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** heroes_pass (Transparent hero layer, target heroes) → final_pass (Sky with outline and shadow, target final)
- **Số pass:** `2`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `paladin → mage → chest → sky → outline_shadow`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `heroes_pass → final_pass`
- **Graph resources:** nodes=`2`, draw commands=`5`, tổng instances=`5`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `8.1521 ms`
- **Thời gian render lần hai (warm/cache):** `2.2413 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `72.5%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `2 pass (transparent heroes → sky/outline final) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/desktop/tc17_outline_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `5847eb5270111b16`
- **SHA-256:** `74b4677ccfef85025c48a36b86198cb6493e7fd077b4b90ccba1275a244419c6`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc17_outline.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận nền trời tím có nhiễu nhẹ; kiếm sĩ, pháp sư và rương xuất hiện đúng vị trí; viền trắng và bóng đổ tím/đen rõ ràng, không có vùng đen hoặc artefact.
- **Graph thực tế:** nodes=2, draw commands=5, instances=5



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `15.6000 ms`
- **Thời gian render lần hai (warm/cache):** `7.0000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `55.1%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `2 pass (transparent heroes → sky/outline final) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/web/tc17_outline_web.bin`
- **Dấu vân tay raw (FNV-1a):** `ea40e775b591176f`
- **SHA-256:** `4d211b675908779d469219c7487c16fbff45a8ae7ec3fa80b55553f47849340a`
- **Ảnh:** ![WebGPU output](../outputs/web/tc17_outline_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Vision xác nhận nền trời tím có nhiễu nhẹ; kiếm sĩ, pháp sư và rương xuất hiện đúng vị trí; viền trắng và bóng đổ tím/đen rõ ràng, không có vùng đen hoặc artefact.
- **Graph thực tế:** nodes=2, draw commands=5, instances=5



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
