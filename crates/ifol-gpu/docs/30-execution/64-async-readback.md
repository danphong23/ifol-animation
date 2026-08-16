# IFOL GPU: hợp đồng readback bất đồng bộ

## Mục tiêu

Readback là một thao tác GPU → CPU dùng chung cho render ảnh, snapshot kiểm thử,
offline rendering, phân tích dữ liệu và các pipeline tính toán. Core không nên
bắt buộc host phải chờ ngay sau khi phát lệnh copy.

## API

`GpuEngine::begin_texture_readback_checked(texture, format)` phát lệnh copy và trả về
`ReadbackTicket`. Ticket sở hữu buffer map, submission index, kích thước ảnh,
bytes-per-pixel và stride đã padding theo `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`.

Host có thể tiếp tục xây dựng hoặc submit công việc khác, sau đó gọi
`ReadbackTicket::resolve_checked(device)`. `resolve_checked` chờ submission hoàn tất, nhận kết
quả map, loại padding từng hàng, unmap buffer và trả `RawTextureReadback` gồm
`bytes`, `width`, `height` và `format` mà host đã khai báo cho copy.

`read_texture_to_raw_with_format_checked` là wrapper đồng bộ của đúng contract
này và trả typed `ReadbackError`. Core không còn cung cấp tuple readback API cũ;
consumer phải dùng `RawTextureReadback` để giữ lại format contract.

## Invariant và giới hạn

- Format phải được truyền rõ ràng; core không đoán format từ `wgpu::Texture`.
- Texture phải có `COPY_SRC` và là texture 2D phù hợp với copy hiện tại.
- Ticket chỉ được resolve một lần vì nó tiêu thụ buffer map.
- Lỗi map hoặc format không hỗ trợ phải trả về lỗi, không panic.
- Chính sách polling, hàng đợi ticket nhiều frame và chuyển dữ liệu sang worker
  thread thuộc lớp host; core chỉ cung cấp ticket sở hữu tài nguyên an toàn.

## Test gate

Test runtime tạo texture 1×1, ghi dữ liệu, bắt đầu readback bất đồng bộ và xác
nhận ticket resolve đúng pixel. Test format width tiếp tục bảo vệ bảng kích
thước byte của các format được hỗ trợ.
