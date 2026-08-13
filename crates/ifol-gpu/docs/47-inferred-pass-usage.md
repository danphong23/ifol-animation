# Inferred usage cho pass built-in

Hazard compiler tự suy ra usage cho semantics đã biết:

- `BufferToBuffer`: source `Read`, destination `Write`;
- `TextureToTexture`: source `Read`, destination `Write`;
- `TextureToTextureAspect`: source `Read`, destination `Write` với
  `TextureAspectRange` tương ứng;
- draw/subgraph có offscreen color hoặc depth attachment: attachment `Write`.

Host vẫn phải khai báo usage cho compute/storage và resource bind qua shader,
vì `wgpu::BindGroup` không cung cấp đủ intent read/write để core suy luận an toàn.
Usage explicit và usage inferred được hợp nhất; duplicate declaration không tạo
duplicate edge.

