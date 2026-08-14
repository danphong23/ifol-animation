# Quy ước tổ chức tài liệu

## Phân loại theo audience

`60-guides/` là public documentation. Người dùng bên ngoài chỉ cần bắt đầu từ
`60-guides/README.md` và đọc learning path ở đó.

Các nhóm `00-foundation`, `10-graph`, `20-resources`, `30-execution`,
`40-memory`, `50-platform`, `70-status` và `80-migration` là design/maintenance
documentation dành cho contributor, maintainer và người cần debug internals.

Tài liệu của `ifol-gpu` được tổ chức theo nhóm kiến trúc, không đặt thành một
danh sách file phẳng ở thư mục `docs/`. `docs/README.md` là mục lục chính và
phải được cập nhật khi thêm, đổi tên hoặc di chuyển tài liệu.

## Các nhóm chính

| Nhóm | Phạm vi | Câu hỏi cần trả lời |
|---|---|---|
| `00-foundation` | Phạm vi, nguyên tắc, test, kế hoạch, tiêu chí hoàn thành | Core được phép biết gì? Task được nghiệm thu thế nào? |
| `10-graph` | Graph khai báo, pass, dependency, hazard, compile, flat plan | Graph được xây dựng và flatten ra sao? |
| `20-resources` | Handle, registry, descriptor, texture, buffer, ownership | Resource được đăng ký, kiểm tra và giải phóng thế nào? |
| `30-execution` | Command, pipeline, compute/copy, submit, readback, profiling | Flat plan được thực thi thế nào? |
| `40-memory` | Pool, frame, upload, readback, deferred destruction | Dữ liệu và lifetime GPU được quản lý ra sao? |
| `50-platform` | Backend, capability, portability, WebGPU/native | Khác biệt thiết bị được cô lập ở đâu? |
| `60-guides` | Hướng dẫn dùng API và mở rộng | Host viết graph, pipeline và extension thế nào? |
| `70-status` | Audit, debt, roadmap, trạng thái triển khai | Đã làm gì và còn thiếu gì? |
| `80-migration` | Tương thích, di trú từ hệ thống cũ | API cũ được thay thế và xóa khi nào? |

## Quy tắc cập nhật

1. Mỗi tài liệu chỉ có một nhóm sở hữu chính. Nếu nội dung liên quan nhiều
   nhóm, đặt tài liệu ở nhóm chứa khái niệm chính và liên kết chéo từ nhóm còn
   lại.
2. Tên file dùng tiền tố số để giữ thứ tự đọc tương đối ổn định; số đã dùng
   không cần dồn lại khi xóa tài liệu.
3. Tài liệu thiết kế chuẩn đặt trong `crates/ifol-gpu/docs`. `.agents/design`
   chỉ giữ vai trò lịch sử, ghi chú điều phối hoặc tài liệu cấp workspace;
   không tạo design mới ở đó nếu design thuộc riêng `ifol-gpu`.
4. Mọi thay đổi làm đổi kiến trúc, public API, invariant hoặc test gate phải
   cập nhật tài liệu cùng task và cùng commit.
5. Không đưa domain như editor video, game scene, ECS, asset database hay
   semantics của shader cụ thể vào graph kernel. Những phần đó thuộc host hoặc
   extension boundary và phải được mô tả ở tài liệu tương ứng.

## Quy trình khi resume goal

Trước mỗi task, đọc theo thứ tự:

1. tài liệu liên quan trong nhóm sở hữu;
2. `00-foundation/13-task-plan.md` và
   `00-foundation/14-definition-of-done.md`;
3. tài liệu status/roadmap mới nhất trong `70-status`;
4. test hiện có và public API trong source.

Task chỉ được xem là hoàn tất khi code, test gate và tài liệu cùng phản ánh
một thiết kế. Nếu source hiện tại mâu thuẫn với tài liệu, phải ghi nhận trong
status và giải quyết bằng một task riêng, không âm thầm quay về cấu trúc cũ.
