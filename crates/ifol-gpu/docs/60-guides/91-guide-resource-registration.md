# Hướng dẫn: đăng ký resource

Host tạo resource bằng `wgpu`, sau đó đăng ký vào `ResourceRegistry` cùng
descriptor mô tả các thuộc tính core cần validate.

| Resource | API |
|---|---|
| Texture view | `insert_texture_with_descriptor` |
| Texture có ownership | `insert_owned_texture` |
| Buffer | `insert_buffer_with_descriptor` |
| Render pipeline | `insert_pipeline_with_layout_descriptor` |
| Compute pipeline | `insert_compute_pipeline_with_layout_descriptor` |
| Bind group | `insert_bind_group_with_descriptor` |
| Mesh | `insert_mesh_with_descriptor` |

```text
create wgpu object
      ↓
create descriptor
      ↓
registry.insert_*_with_descriptor(...)
      ↓
retain typed handle
      ↓
graph command references handle
```

## Ownership

`insert_texture_with_descriptor` chỉ đăng ký view. Dùng
`insert_owned_texture` khi core cần texture object cho copy, resolve, readback
hoặc deferred destruction. Graph không sở hữu resource.

## Replacement và removal

Mỗi replacement làm tăng resource version để cache/bundle invalidation hoạt động.
Dùng accessor (`texture`, `buffer`, `pipeline`, `mesh`, `bind_group`) để lookup
và API `remove_*`/deferred lifecycle để giải phóng. Không truy cập raw map nội bộ.
