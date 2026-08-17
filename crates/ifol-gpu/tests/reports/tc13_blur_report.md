# Báo cáo: TC13 - Gaussian blur hai chiều và depth of field điện ảnh

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc13_blur.json`
- **Graph fingerprint (FNV-1a):** `4f37a8fd4102496e`
- **Mô tả test case:** Render hậu cảnh rừng canonical, blur Gaussian theo hai hướng qua target ping-pong, rồi ghép ba đối tượng tiền cảnh sắc nét.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `gaussian_blur_separable.wgsl`, `sky_composite.wgsl`, `star_particles_sprite.wgsl`, `texture_blit.wgsl`
- **Asset/input:** `canonical_bg_forest_props1.png`, `canonical_sprites_heroes.png`, `canonical_sprites_items.png`, `canonical_tc085_noise.png`, `canonical_tc085_props.png`
- **Chính sách input:** Dùng PNG canonical để Desktop/WebGPU giải mã cùng một input byte-level.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** background_scene (Forest environment, target background_a) → blur_horizontal_pass (Horizontal 9-tap Gaussian blur, target blur_b) → blur_vertical_pass (Vertical 9-tap Gaussian blur, target background_a) → final_composite (Blurred background and sharp foreground, target final)
- **Số pass:** `4`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `forest_sky → forest_wisps → tree_left → tree_center → tree_right → blur_horizontal → blur_vertical → background_blit → paladin_foreground → archer_foreground → chest_foreground`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `background_scene → blur_horizontal_pass → blur_vertical_pass → final_composite`
- **Graph resources:** nodes=`4`, draw commands=`11`, tổng instances=`50`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `11.4738 ms`
- **Thời gian render lần hai (warm/cache):** `4.5143 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `60.7%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `4 pass (background → blur H → blur V → final) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/desktop/tc13_blur_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `eddefd536825f6e3`
- **SHA-256:** `31f0a8c0eb579826584318ba6a1a034d514c5d26797d0b63dd0068a99e98f4c4`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc13_blur.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Hậu cảnh rừng và wisps được blur mềm theo hai hướng; paladin, archer và chest foreground giữ nét; không có banding hoặc artifact rõ ràng.
- **Graph thực tế:** nodes=4, draw commands=11, instances=None



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `23.1000 ms`
- **Thời gian render lần hai (warm/cache):** `13.2000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `42.9%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `4 pass (background → blur H → blur V → final) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Dữ liệu raw:** `tests/outputs/web/tc13_blur_web.bin`
- **Dấu vân tay raw (FNV-1a):** `eddefd536825f6e3`
- **SHA-256:** `31f0a8c0eb579826584318ba6a1a034d514c5d26797d0b63dd0068a99e98f4c4`
- **Ảnh:** ![WebGPU output](../outputs/web/tc13_blur_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** ĐẠT: Web có cùng nền rừng blur hai hướng và ba đối tượng foreground sắc nét; không có ping-pong state leak hoặc artifact rõ ràng.
- **Graph thực tế:** nodes=4, draw commands=11, instances=None



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `ĐẠT` |
| Số byte khác nhau | `0` |
| Số pixel khác nhau | `0` |
| Sai số kênh màu lớn nhất | `0/255` |
| Khác biệt màu/presentation | `KHÔNG` |
| Số pixel non-background Desktop/Web | `KHÔNG ÁP DỤNG` |
| Bounding box Desktop | `KHÔNG ÁP DỤNG` |
| Bounding box WebGPU | `KHÔNG ÁP DỤNG` |
| Bounding box non-background giống nhau | `ĐẠT` |
| Số pixel mask khác nhau | `0` (ngưỡng `0`) |
| Parity cấu trúc không phụ thuộc màu | `ĐẠT` |
| Cache giữ nguyên output cold/warm ở cả hai môi trường | `ĐẠT` |
| Validation/fallback contract không panic | `ĐẠT` |
| Đúng mô tả test case | `ĐẠT` |

**Kết luận:** `ĐẠT - output giống tuyệt đối từng byte.`

## 5. Phân tích hiệu suất

Các giá trị trên đo thời gian thực thi graph, submit lệnh và chờ GPU hoàn tất;
không bao gồm khởi tạo device/pipeline hoặc readback. Vì vậy `cold` ở đây là
lần execute đầu sau khi resource/pipeline đã được tạo, không phải cold start
của toàn bộ ứng dụng. Giá trị dưới `1 ms` tương đương microsecond và cần được
đọc theo đơn vị đó khi phân tích.
