# Inferred usage cho pass built-in

Hazard compiler tự suy ra usage cho semantics đã biết:

- `BufferToBuffer`: source là `Read`, destination là `Write`;
- `TextureToTexture`: source là `Read`, destination là `Write`;
- draw/subgraph có offscreen color hoặc depth attachment: attachment là
  `Write`.

Host vẫn phải khai báo usage cho compute/storage và các resource bind qua shader,
vì `wgpu::BindGroup` không cung cấp đủ intent read/write để core suy luận an
toàn. Usage explicit và usage inferred được hợp nhất; duplicate declaration
không tạo duplicate edge.
