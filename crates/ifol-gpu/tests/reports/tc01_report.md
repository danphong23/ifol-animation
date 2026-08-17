# Báo cáo: TC01 - Empty Render

Đây là báo cáo kiểm thử hai môi trường dùng chung manifest và graph contract.

## 1. Mô tả và graph dùng chung

- **Manifest:** `tests/shared_assets/manifests/tc01_empty.json`
- **Graph fingerprint (FNV-1a):** `e1b51be2d4727286`
- **Mô tả test case:** Clear an 800x600 offscreen target to a uniform light gray without garbage pixels.
- **Target:** `800x600`, `Rgba8Unorm`
- **Desktop/Web dùng cùng manifest fingerprint:** `PASS`

## 2. Desktop

- **Cold render:** `1.6666999999999998 ms`
- **Warm/cached render:** `0.415 ms`
- **Raw output:** `tests/outputs/desktop/tc01_empty_desktop.bin`
- **Raw fingerprint (FNV-1a):** `56e43abaf9ecc325`
- **SHA-256:** `d64fc378fbb5847dd03659fcf2681adfc74ff4b3df8d4794dea3dbf88704db5f`
- **Ảnh:** ![Desktop output](../outputs/desktop/tc01_empty.png)
- **Đánh giá nội dung:** `PASS`

## 3. WebGPU

- **Cold render:** `3.300000011920929 ms`
- **Warm/cached render:** `1 ms`
- **Raw output:** `tests/outputs/web/tc01_empty_web.bin`
- **Raw fingerprint (FNV-1a):** `56e43abaf9ecc325`
- **SHA-256:** `d64fc378fbb5847dd03659fcf2681adfc74ff4b3df8d4794dea3dbf88704db5f`
- **Ảnh:** ![WebGPU output](../outputs/web/tc01_empty_web.png)
- **Đánh giá nội dung:** `PASS`

## 4. So sánh và kết luận

| Tiêu chí | Kết quả |
| --- | --- |
| Graph/manifest giống nhau | `PASS` |
| Kích thước raw giống nhau | `PASS` |
| Raw bytes giống tuyệt đối | `PASS` |
| Số byte khác nhau | `0` |
| Đúng mô tả test case | `PASS` |

**Kết luận:** `PASS - Desktop và WebGPU cho output canonical giống nhau từng byte.`
