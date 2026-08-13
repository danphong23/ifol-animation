# IFOL GPU: typed readback errors

## Contract

Readback chỉ còn API checked trả `ReadbackError` có cấu trúc. Các wrapper trả
`&'static str` đã bị loại khỏi core để không làm mất nguyên nhân lỗi.

`ReadbackError` phân biệt:

- `InvalidExtent`: texture có kích thước zero;
- `UnsupportedFormat(format)`: core chưa biết bytes-per-pixel của format;
- `ArithmeticOverflow`: layout row/ buffer size overflow;
- `MapFailed`: GPU không map được staging buffer;
- `AccessFailed`: mapped range không truy cập được hoặc không đủ dữ liệu.

`ReadbackTicket::resolve_checked` cũng kiểm tra row bounds trước khi copy bytes,
không panic khi dữ liệu staging không khớp layout kỳ vọng. Host có thể tự
chuyển typed error thành message theo policy giao diện của mình.

## Phạm vi

API nhận format do host khai báo vì `wgpu::Texture` không expose descriptor đầy
đủ sau khi tạo. Core chưa tự chuyển đổi depth/stencil hoặc compressed format.

## Test gate

Test async readback hiện tại vẫn pass; test mới xác nhận format không hỗ trợ trả
`ReadbackError::UnsupportedFormat` trước submit.
