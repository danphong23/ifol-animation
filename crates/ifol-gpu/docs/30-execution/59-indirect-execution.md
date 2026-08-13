# Indirect draw và dispatch

Graph hỗ trợ:

- `DrawAction::Indirect`: đọc cấu trúc draw-indirect 16 byte;
- `DrawAction::IndexedIndirect`: đọc cấu trúc indexed-draw 20 byte và yêu cầu
  mesh có index buffer;
- `ComputeCommand::new_indirect`: đọc cấu trúc dispatch-indirect 12 byte.

Compiler validate buffer tồn tại, offset căn 4 byte, range nằm trong buffer và
descriptor có usage `INDIRECT` khi metadata descriptor có sẵn. Graph tự khai báo
byte range đọc của indirect buffer để hazard compiler giữ đúng thứ tự với các
node ghi argument buffer.
