# IFOL GPU: typed texture save errors

`GpuEngine::save_texture_to_file_checked` trong `backend/texture_save.rs` là
API checked cho offline rendering
và visual snapshot. Nó dùng readback RGBA8 sRGB legacy contract, sau đó báo lỗi
typed:

API này thuộc feature `image-encode`, được bật mặc định; core GPU có thể được
build bằng `--no-default-features` mà không kéo dependency `image`.

- `TextureSaveError::Readback(ReadbackError)` khi GPU readback lỗi;
- `TextureSaveError::CreateDirectory` khi không tạo được thư mục cha;
- `TextureSaveError::Encode` khi image backend không ghi được file.

Core chỉ cung cấp `save_texture_to_file_checked` với lỗi typed. Core không tự
chọn format từ file extension; extension chỉ do image backend xử lý khi encode.

Test edge case dùng một file thường làm parent path và xác nhận encode failure
được giữ dưới dạng typed error thay vì bị nuốt thành một trạng thái mơ hồ.
