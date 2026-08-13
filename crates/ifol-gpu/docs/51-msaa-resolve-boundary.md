# MSAA và resolve boundary

Render compiler hiện dùng `sample_count: 1`, không có resolve target và chưa có
subresource/aspect model. Vì vậy `execute_checked` từ chối color/depth texture
có `sample_count != 1` bằng `UnsupportedSampleCount` trước khi encode.

Đây là behavior có chủ ý để không gửi graph MSAA vào backend với attachment và
pipeline không tương thích. MSAA/resolve sẽ cần API riêng mô tả:

- multisampled render attachment;
- single-sample resolve target;
- format/sample compatibility;
- load/store và lifetime của cả hai resource.

Hiện tại không được coi lỗi này là thiếu capability của GPU; đây là giới hạn
execution path của core.
