# Báo cáo: TC55 - Bloom Dual Kawase phân cấp

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `../shared_assets/manifests/tc55_dual_kawase.json`
- **Graph fingerprint (FNV-1a):** `2a88441e6a8ac270`
- **Mô tả test case:** Tách mage, giảm độ phân giải qua Kawase 8 mẫu rồi composite bloom và foreground sắc nét.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `dual_kawase.wgsl`, `texture_blit.wgsl`
- **Asset/input:** `canonical_bg_scifi.png`, `canonical_sprites_heroes.png`
- **Chính sách input:** Desktop và WebGPU dùng texture PNG canonical; các target 800x600 và 400x300 dùng cùng graph contract.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** extract_pass (Mage extraction, target mage) → downsample_pass (400x300 Kawase downsample, target downsample) → composite_pass (Background, bloom and sharp mage, target final)
- **Số pass:** `3`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `kawase_extract_mage → kawase_downsample → kawase_background → kawase_bloom → kawase_foreground`
- **Sampler contract:** `{"address_mode_u": "clamp-to-edge", "address_mode_v": "clamp-to-edge", "address_mode_w": "clamp-to-edge", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `extract_pass → downsample_pass → composite_pass`
- **Graph resources:** nodes=`3`, draw commands=`5`, tổng instances=`5`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `5.6800 ms`
- **Thời gian render lần hai (warm/cache):** `1.9426 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `65.8%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `3 pass extract → 400x300 Kawase → composite + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `../outputs/desktop/tc55_dual_kawase_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `ebbdaaeff4dac486`
- **SHA-256:** `87fb44630d8997046d378cd95e249439a33c9fc89c6cd66074a59c8c104d39d0`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc55_dual_kawase.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Desktop hiển thị mage sắc nét trên nền Sci-Fi, cùng vùng glow bloom mềm từ pass downsample; foreground vẫn giữ rõ.
- **Graph thực tế:** nodes=3, draw commands=5, instances=5



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `23.7000 ms`
- **Thời gian render lần hai (warm/cache):** `3.1000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `86.9%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `3 pass extract → 400x300 Kawase → composite + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `../outputs/web/tc55_dual_kawase_web.bin`
- **Dấu vân tay raw (FNV-1a):** `ebbdaaeff4dac486`
- **SHA-256:** `87fb44630d8997046d378cd95e249439a33c9fc89c6cd66074a59c8c104d39d0`
- **Ảnh:** ![WebGPU output](../outputs/web/tc55_dual_kawase_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** WebGPU hiển thị cùng mage sắc nét, nền Sci-Fi và bloom mềm; foreground không bị mất hoặc sai tỷ lệ.
- **Graph thực tế:** nodes=3, draw commands=5, instances=5



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
