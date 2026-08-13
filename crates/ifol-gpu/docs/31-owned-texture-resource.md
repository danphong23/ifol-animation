# IFOL GPU: owned texture resource

`ResourceRegistry::insert_owned_texture` lưu `wgpu::Texture` thật, tạo view
compatibility và lưu descriptor cùng một handle. `owned_texture` cho phép các pass
copy/resolve truy cập texture object; API cũ `insert_texture` vẫn chỉ là view-only
compatibility path.

`remove_owned_texture` xóa đồng bộ texture view, descriptor và ownership metadata,
đồng thời tăng resource version để invalidation cache không giữ artifact cũ.
