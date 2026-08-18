# Báo cáo: TC42 - Bloom HDR toàn khung

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và hợp đồng graph.

## 1. Mô tả và graph dùng chung

- **Manifest:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/shared_assets/manifests/tc42_hdr_bloom.json`
- **Graph fingerprint (FNV-1a):** `ded6885c267c0133`
- **Mô tả test case:** Tách mage, lấy vùng phát sáng, rồi cộng bloom lên nền Sci-Fi canonical trong composite nhiều pass.
- **Target:** `800x600`, `Rgba8UnormSrgb`
- **Shader/WGSL:** `chroma_key_cropped.wgsl`, `emissive_bloom.wgsl`, `texture_blit.wgsl`
- **Asset/input:** `canonical_bg_scifi.png`, `canonical_sprites_heroes.png`
- **Chính sách input:** Desktop và WebGPU dùng hai PNG canonical: canonical_sprites_heroes.png và canonical_bg_scifi.png.
- **Depth/stencil:** `Không áp dụng`
- **Chuỗi pass:** chroma_pass (không tên, target mage) → bloom_pass (không tên, target bloom) → final_pass (không tên, target final)
- **Số pass:** `3`
- **Độ sâu graph:** `KHÔNG ÁP DỤNG`
- **Hierarchy:** `Không khai báo`
- **Thứ tự operation sau flatten:** `chroma_extract_mage → emissive_extract → background → bloom_add → mage_over`
- **Sampler contract:** `{"address_mode_u": "repeat", "address_mode_v": "repeat", "address_mode_w": "repeat", "mag_filter": "linear", "min_filter": "linear", "mipmap_filter": "linear"}`
- **Thứ tự layer kỳ vọng:** `chroma_pass → bloom_pass → final_pass`
- **Graph resources:** nodes=`3`, draw commands=`5`, tổng instances=`5`, procedural particles=`Không khai báo`
- **Node pool contract:** `Không áp dụng`
- **Error/fallback contract:** `Không áp dụng`
- **Desktop/Web dùng cùng manifest fingerprint:** `ĐẠT`

## 2. Môi trường Desktop

- **Thời gian render lần đầu (cold):** `9.0538 ms`
- **Thời gian render lần hai (warm/cache):** `4.1849 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `53.8%`
- **Adapter/backend:** `Intel(R) Iris(R) Xe Graphics` / `Vulkan`
- **Phạm vi timing:** `3 pass (chroma → bloom → composite) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU`
- **Dữ liệu raw:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/outputs/desktop/tc42_hdr_bloom_desktop.bin`
- **Dấu vân tay raw (FNV-1a):** `140bdc9027bc5a74`
- **SHA-256:** `7fbe3b4cbdb2e9ea45a25a681d79b33b2405f98037c2b712d627d3ffe69433f0`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc42_hdr_bloom.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Nền Sci-Fi, mage trung tâm và bloom lan rộng không bị cắt vuông; thứ tự layer đúng.
- **Graph thực tế:** nodes=3, draw commands=5, instances=5



## 3. Môi trường WebGPU

- **Thời gian render lần đầu (cold):** `9.3000 ms`
- **Thời gian render lần hai (warm/cache):** `5.6000 ms`
- **Số lần warm được đo:** `1`
- **Output cold và warm giống nhau:** `True`
- **Speedup cold → warm:** `39.8%`
- **Adapter:** `gen-12lp`
- **Phạm vi timing:** `3 pass chroma → bloom → composite + submit queue + onSubmittedWorkDone; không gồm khởi tạo device/pipeline và readback`
- **Phạm vi cô lập/cache:** `Resource của TC được hủy sau khi hoàn tất; không xóa cache nội bộ của browser/driver/GPU`
- **Dữ liệu raw:** `C:/Users/abc/.AI/Code/ifol-animation/crates/ifol-gpu/tests/outputs/web/tc42_hdr_bloom_web.bin`
- **Dấu vân tay raw (FNV-1a):** `97d1d51362834813`
- **SHA-256:** `9f972df5e7bb52205a24feffab35dc3030d5ca467ffa193d0623cbf427773d3d`
- **Ảnh:** ![WebGPU output](../outputs/web/tc42_hdr_bloom_web.png)
- **Đánh giá nội dung:** `ĐẠT`
- **Đánh giá bằng vision:** Nền Sci-Fi, mage trung tâm và bloom lan rộng không bị cắt vuông; thứ tự layer đúng.
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
