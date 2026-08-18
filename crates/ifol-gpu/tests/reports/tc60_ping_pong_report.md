# Báo cáo: TC60 - Vòng phản hồi ping-pong

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `../shared_assets/manifests/tc60_ping_pong.json`
- **Graph fingerprint (FNV-1a):** `861e27bfb471246e`
- **Mô tả test case:** Render wizard rồi tích lũy phản hồi zoom xác định qua hai target ping và pong luân phiên.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `ping_pong_blit.wgsl`
- **Asset/input:** `canonical_sprites_heroes.png`
- **Chính sách input:** Desktop và WebGPU dùng cùng sprite PNG canonical, cùng target 800x600 và cùng sequence 16 bước feedback.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** seed_pass (Wizard seed, target ping) → feedback_passes (8 ping-pong cycles, target ping/pong) → final_pass (Final copy, target final)
- **Số pass:** `3`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `feedback_seed → feedback_zoom_out → feedback_zoom_in → feedback_final_copy`
- **Sampler contract:** `{"address_mode_u": "clamp-to-edge", "address_mode_v": "clamp-to-edge", "address_mode_w": "clamp-to-edge", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `seed_pass → feedback_passes → final_pass`
- **Graph resources:** nodes=`3`, draw commands=`20`, tổng instances=`18`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `6.7678 ms`
- **Thời gian render lần hai (warm/cache):** `4.7808 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `29.4%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `18 graph executions (seed + 16 feedback + final copy) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `../outputs/desktop/tc60_ping_pong_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `b057a9680b0253a7`
- **SHA-256:** `2d5d414f9b217cc83ae2c8cf294a2fbc155ea1a12e4a3295b28bfd53095030e2`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc60_ping_pong.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Desktop hiển thị wizard trung tâm với nhiều bóng mờ đồng tâm lan ra; độ mờ giảm dần và nền không bị đen hoặc vỡ alpha.
- **Graph thực tế:** nodes=3, draw commands=20, instances=4



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `22.8000 ms`
- **Thời gian render lần hai (warm/cache):** `3.3000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `85.5%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `18 render passes (seed + 16 feedback + final copy) + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `../outputs/web/tc60_ping_pong_web.bin`
- **Dấu vân tay raw (FNV-1a):** `dec9302bd5e0592e`
- **SHA-256:** `b170dcec906039f0384d24393f389fdbb47081fd65b23727c804990074ba921d`
- **Ảnh:** ![WebGPU output](../outputs/web/tc60_ping_pong_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** WebGPU hiển thị cùng wizard và cùng chuỗi echo feedback; vị trí, hướng lan và độ mờ tương ứng, khác biệt chỉ ở biên/rasterization nhỏ.
- **Graph thực tế:** nodes=3, draw commands=20, instances=4



## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `ĐẠT` |
| Kích thước dữ liệu raw giống nhau | `ĐẠT` |
| Byte raw giống tuyệt đối | `KHÔNG ĐẠT` |
| Số byte khác nhau | `6165` |
| Số pixel khác nhau | `4032` |
| Sai số kênh màu lớn nhất | `5/255` |
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
