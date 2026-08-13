# IFOL GPU: validation cấu trúc texture descriptor

`TextureResourceDescriptor::validate` chặn các descriptor chắc chắn không thể
hợp lệ trước khi registry nhận resource:

- width/height, layer, mip count, sample count và usage không được rỗng/zero;
- mip count không vượt quá số mức mip tối đa suy ra từ extent;
- sample count phải là lũy thừa của hai.

Đây chỉ là validation cấu trúc, không phải capability matrix của adapter. Sample
count thực tế, format feature, usage được phép và resolve support vẫn cần
device-aware validation ở bước tiếp theo.

Test hồi quy bao phủ mip count vượt extent và sample count `3`; cả hai đều trả
typed `ResourceDescriptorError` và không mutate registry.

