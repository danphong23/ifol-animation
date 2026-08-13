# IFOL GPU: Validation, error và diagnostics

## Nguyên tắc

Input không hợp lệ phải tạo structured error trước khi GPU submit nếu có thể. Resource thiếu không được âm thầm bỏ qua, vì việc bỏ qua draw có thể che giấu graph hỏng và tạo output không đầy đủ.

## Nhóm error

API mục tiêu nên phân biệt:

- lỗi khởi tạo instance/adapter/device;
- lỗi feature hoặc limit không hỗ trợ;
- lỗi tạo resource và stale handle;
- lỗi dependency, cycle của graph;
- lỗi format/sample/attachment không tương thích;
- lỗi pipeline/layout/binding không tương thích;
- lỗi upload, readback, surface và submission.

Error nên chỉ ra pass, node, resource handle, giá trị kỳ vọng và giá trị thực tế nếu có. `&'static str` không đủ cho public error boundary.

## Mức validation

- `Off`: ít check, dành cho release path đã tin cậy;
- `Basic`: check handle, range, slot và attachment;
- `Strict`: check dependency, usage, pipeline compatibility và diagnostics chi tiết.

Implementation có thể thay đổi, nhưng hành vi validation phải được ghi rõ và deterministic.

## Diagnostics

Core nên expose label, debug marker, command trace, resource statistic và CPU/GPU timing hook tùy chọn. Diagnostics không được thay đổi rendering semantic.

## Logging và panic policy

Library code không được panic với input không hợp lệ thông thường. `unwrap()` chỉ chấp nhận trong test hoặc invariant nội bộ không thể xảy ra, và phải có lý do được ghi rõ. Public execution phải trả về error hoặc trạng thái device/surface có thể xử lý.
