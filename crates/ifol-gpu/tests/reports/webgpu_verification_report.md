# Báo cáo WebGPU và canonical parity

**Ngày kiểm thử:** 17/08/2026<br>
**Web runner:** `tests/web_runner/` trên Codex In-app Browser<br>
**Kích thước canonical target:** `800 × 600`
**Canonical format:** `Rgba8Unorm`

## Kết quả

| Hạng mục | Kết quả |
|---|---|
| WebGPU runner | 7/7 test case pass |
| Desktop canonical probe | Pass |
| Web canonical probe | Pass |
| Raw output size | 1,920,000 bytes mỗi môi trường |
| Raw byte differences | 0 |
| Maximum byte delta | 0 |

Canonical probe dùng cùng clear value `[0.03, 0.04, 0.07, 1.0]`, cùng kích
thước và cùng `Rgba8Unorm`. Hai raw output tạm của Desktop và Web giống nhau
từng byte. SHA-256 của cả hai output là
`4F2AB7130334569606F07A9F0304A2A39DDFCC89C2563B54F8B4384777C813E2`.

## Thời gian đo được

| Môi trường | Thời gian probe |
|---|---:|
| Desktop native | 1.5224 ms |
| WebGPU browser | 23.90 ms |

Đây là thời gian đo trên hai môi trường khác nhau, không phải tiêu chí phải
bằng nhau. Nó chỉ dùng làm baseline phát hiện regression trong cùng môi trường.

## Giới hạn của kết quả

Ảnh PNG hiện tại của 7 test cũ vẫn render trực tiếp vào surface WebGPU và
Desktop, nên không được gọi là pixel-perfect canonical output. So sánh ảnh
surface hiện tại cho thấy khác biệt do surface format/presentation; nó không
phủ định canonical probe.

Kết luận đúng ở phase này là: **canonical offscreen path đã tạo output giống
nhau cho probe; parity pixel-perfect của toàn bộ TC98–TC105 vẫn chưa được
chứng minh và sẽ được xử lý khi từng test chuyển sang canonical target.**
