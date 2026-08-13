# Transient texture pool

`TextureCache` cũ đã được đổi semantics thành `TransientTexturePool`. Đây là
free-list exact-match, không phải LRU hay bộ quản lý áp lực VRAM.

`TextureDescriptorKey` phải khớp toàn bộ thuộc tính ảnh hưởng compatibility:

- width/height/depth hoặc số layer;
- format và usage;
- mip count và sample count;
- dimension 1D/2D/3D.

Khi release, host truyền `last_use: SubmissionId`. Handle chỉ được acquire lại
khi `SubmissionTracker.completed()` đã đạt submission đó. Release trùng handle
bị từ chối. `drain_completed` trả các handle đã hoàn tất để resource manager có
thể giải phóng backing texture khi cần eviction.

Pool chỉ quản lý handle/lifetime policy; việc tạo, sở hữu và destroy
`wgpu::Texture` vẫn thuộc resource manager/registry.
