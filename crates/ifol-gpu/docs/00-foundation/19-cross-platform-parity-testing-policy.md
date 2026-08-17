# Chính sách kiểm thử parity Desktop/Web

## Các mức kết luận

Một test case phải phân biệt rõ các mức sau:

| Mức | Ý nghĩa |
|---|---|
| Vision parity | Ảnh đúng mô tả và không có artifact rõ ràng |
| Structural parity | Graph, vị trí, kích thước, layer, depth và mask hình học giống |
| Raw parity | Raw RGBA readback giống từng byte |
| File parity | File canonical sau encoder có cùng nội dung/byte hash |

`ĐẠT CÓ ĐIỀU KIỆN` chỉ được dùng khi vision/structural đạt nhưng raw hoặc file
parity còn khác. Không được gọi trường hợp này là pixel-perfect.

## Manifest và fingerprint

Mỗi TC cần một manifest dùng chung cho Desktop và Web. Manifest phải mô tả:

- target width/height/format;
- operation, shader, asset, tham số và thứ tự submit;
- depth/blend/sampler contract nếu có;
- expected output và ngưỡng so sánh.

Fingerprint tối thiểu là hash của manifest. Khi canonical asset pipeline hoàn
thiện, chứng nhận đầy đủ phải bổ sung hash của shader bytes và canonical asset
bytes để chứng minh toàn bộ input giống nhau.

## Quy trình một batch

```text
Đọc report cũ và manifest
        ↓
Chạy Desktop 1–3 TC
        ↓
Chạy đúng manifest trên WebGPU
        ↓
Raw hash + byte/pixel diff
        ↓
Vision review ảnh của từng môi trường
        ↓
Đánh giá mô tả và graph
        ↓
Report tiếng Việt hoàn chỉnh
        ↓
PASS → commit riêng
```

Không chạy toàn bộ suite cùng lúc khi có nguy cơ làm lag máy. Không suy luận
WebGPU từ Desktop; nếu Web chưa chạy phải ghi `CHƯA CHẠY`.

## Timing

Mỗi report ghi riêng cho Desktop và WebGPU:

- cold render;
- warm/cached render;
- readback nếu đo riêng;
- adapter/backend;
- phạm vi timer.

Phạm vi chuẩn hiện tại là `execute + submit + GPU wait`, không bao gồm khởi
tạo device/pipeline hoặc readback. Giá trị dưới `1 ms` phải ghi thêm µs. Log
toàn bộ browser runner có thể bao gồm initialization và không được nhầm với
graph render timing.

## Vision và raw comparison

Vision kiểm tra ảnh thực tế theo mô tả TC: bố cục, layer/depth, artifact, alpha,
viền và màu khi màu là yêu cầu của TC. Raw verifier kiểm tra kích thước, hash,
byte diff, pixel diff, sai số kênh lớn nhất, bounding box và non-background mask.

Ảnh dùng để vision phải được tạo từ canonical raw readback khi có thể. Canvas
presentation của browser chỉ dùng để preview, không dùng làm source of truth.

## Diễn giải sai khác

- Sai khác toàn ảnh/background: kiểm tra format, clear color, color encoding.
- Sai khác ở sprite/viền: kiểm tra asset decode, sampler, sRGB và chroma-key.
- Sai khác layer/depth/bounding box: kiểm tra graph, coordinate, depth compare
  và submission order.
- Raw khác nhưng vision/structure đạt: kết luận là parity có điều kiện, không
  che giấu bằng nhãn pass tuyệt đối.

## Canonical path nằm ngoài core

Các test Desktop/Web trong thư mục này chứng minh contract thực thi và raw
readback của `ifol-gpu`; chúng không chuyển `ifol-gpu` thành image/video
pipeline. Test harness có thể dùng fixture PNG để kiểm soát input, nhưng sản
phẩm phải để tầng asset/media quản lý decoder, canonical asset bytes, color
policy và encoder. Khi tầng ngoài chạy export thật, cần ghi thêm hash của input,
raw frame và file cuối cùng để phân biệt lỗi decode, render và encode.

## Trạng thái hiện tại

- TC01: raw parity tuyệt đối giữa Desktop/WebGPU.
- TC02: vision/structural parity đạt; raw còn sai khác màu/pixel.
- TC03: vision/structural/depth parity đạt; raw còn sai khác màu/alpha.
- TC04: vision/structural/depth và raw parity tuyệt đối với fixture canonical.
- TC05: vision/structural và raw parity tuyệt đối với chuỗi pass A→B→C và
  fixture canonical.
- TC06: pool invariant `100 → remove 99 → 1 survivor`, vision/structural và raw
  parity tuyệt đối với fixture canonical.

Các kết quả trên là bằng chứng kiểm thử hiện tại, chưa phải chứng nhận rằng
canonical export đã bit-exact trên mọi GPU/backend.
