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

Vì vậy một test parity đầy đủ phải tách ba nguồn sai khác:

1. **Decode/input:** hai host có tạo đúng cùng canonical bytes hay không;
2. **Render:** cùng graph, shader, pipeline, resource contract và raw readback
   có cho cùng frame hay không;
3. **Encode/output:** hai host có dùng cùng encoder/profile/metadata hay không.

Khi tầng ngoài triển khai canonical export, workflow chuẩn phải có ownership
rõ ràng:

```text
asset/media layer -> canonical decoded bytes + hash
graph/shader layer -> shared graph/pipeline contract
ifol-gpu -> execute + raw readback
export layer -> canonical encoder + metadata + file hash
```

`ifol-gpu` không tự động trở thành decoder, color-management service hoặc media
encoder. Thêm API decode/encode vào core, kể cả chỉ để test hoặc benchmark gọi,
là mở rộng sai boundary.

Chỉ mục thứ hai thuộc phạm vi chứng nhận trực tiếp của `ifol-gpu`. Hai mục còn
lại thuộc tầng ngoài và phải có report riêng khi canonical export được triển
khai. JPEG, PNG, WebP và video đều có thể được hỗ trợ ở tầng ngoài mà không làm
thay đổi boundary của core.

## Trạng thái hiện tại

- TC01: raw parity tuyệt đối giữa Desktop/WebGPU.
- TC02: vision/structural parity đạt; raw còn sai khác màu/pixel.
- TC03: vision/structural/depth parity đạt; raw còn sai khác màu/alpha.
- TC04: vision/structural/depth và raw parity tuyệt đối với fixture canonical.
- TC05: vision/structural và raw parity tuyệt đối với chuỗi pass A→B→C và
  fixture canonical.
- TC06: pool invariant `100 → remove 99 → 1 survivor`, vision/structural và raw
  parity tuyệt đối với fixture canonical.
- TC07: graph đệ quy 5 cấp, thứ tự flatten E → D → C → B → A, vision/structural
  và raw parity tuyệt đối với canonical crop và sampler `nearest`.
- TC08: một node với 2 draw command và 10.000 procedural instances, vision/
  structural và raw parity tuyệt đối với background canonical và sampler
  `nearest`.
- TC08.5: cảnh đêm 2 pass `scene → final`, mặt trăng, 4 lớp mây, sao và bloom;
  vision/structural parity đạt, raw còn khác 1 byte ở 1 pixel với sai số kênh
  tối đa `1/255`, nên kết luận là `ĐẠT CÓ ĐIỀU KIỆN`.
- TC09: cùng graph được chạy cold và 10 warm lần trên mỗi môi trường; output
  cold/warm giữ nguyên và raw parity Desktop/WebGPU đạt tuyệt đối. Timing warm
  được báo cáo để quan sát cache, không phải ngưỡng phần cứng cố định.
- TC10: Desktop xác nhận `MissingBindGroup(999999)` trả typed error không panic;
  Web mirror cùng error contract, fallback magenta và raw parity đạt tuyệt đối.
- TC11: ba pass `left → right → final` với hai target offscreen 400x600 và
  split compositor, vision/structural parity và raw parity đạt tuyệt đối.
- TC12: một pass gồm sky canonical và 5 sprite chroma-key; vision/structural
  parity đạt, raw còn khác 4 byte ở 4 pixel với sai số kênh tối đa `1/255`,
  nên kết luận là `ĐẠT CÓ ĐIỀU KIỆN`.
- TC13: bốn pass `background → blur H → blur V → final`, hai target ping-pong,
  11 draw command; vision/structural parity và raw parity đạt tuyệt đối, cold/
  warm output không đổi trên Desktop/WebGPU.

Các kết quả trên là bằng chứng kiểm thử hiện tại, chưa phải chứng nhận rằng
canonical export đã bit-exact trên mọi GPU/backend.
