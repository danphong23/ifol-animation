# Ý định hiện tại và workflow nâng cấp

Tài liệu này là nguồn chỉ dẫn ngắn gọn cho các chat/task tiếp theo. Nó ưu tiên
ý định hiện tại hơn các backlog cũ.

## Mục tiêu

`ifol-gpu` là GPU execution substrate generic:

- cung cấp resource contract, handle, lifetime và capability;
- nhận graph khai báo và flatten graph thành kế hoạch thực thi;
- validate dependency, hazard và command;
- encode render, compute, copy và submit GPU work;
- cung cấp readback raw và typed error;
- cho phép extension đăng ký operation mà không đưa domain semantics vào core.

## Không thuộc core

Các phần sau thuộc host/higher layer:

- scene, ECS, animation, timeline và editor;
- asset database, image/video/audio codec và file export;
- material semantics và shader compiler/reflection;
- color management, color science, HDR policy, tone mapping và grading;
- application policy, present policy và domain scheduling.

## Ranh giới màu

Core chỉ được biết `wgpu::TextureFormat` như physical GPU format và các giá trị
clear của attachment. Core không được tự suy luận `sRGB`, P3, ACES, BT.709,
PQ, HLG hoặc màu hiển thị.

Readback phải trả raw bytes cùng kích thước và format thực tế. Chuyển đổi màu,
đóng gói ảnh và encode video phải nằm ở tầng ngoài.

Core không chứa API lưu file và không phụ thuộc `image`. Test/example support có
thể tự encode output từ raw readback ở dev-dependency hoặc higher layer.

## Nguyên tắc thay đổi

1. Không viết lại graph đang hoạt động.
2. Tách file trước, giữ nguyên behavior.
3. Mỗi task chỉ có một responsibility.
4. Không đổi API, color policy và module structure trong cùng một task.
5. Không xóa compatibility path nếu chưa có migration task và test downstream.
6. Không tạo file chỉ để đạt một cây thư mục đẹp; responsibility phải độc lập.

## Vòng đời một task

```text
Inspect
  ↓
Một thay đổi nhỏ, không đổi semantics
  ↓
cargo fmt --all -- --check
  ↓
cargo check -p ifol-gpu
  ↓
unit test liên quan
  ↓
regression test
  ↓
PASS → commit
FAIL → sửa và chạy lại, không commit
```

Commit không được chứa code đang fail. Test không được bị xóa hoặc làm yếu chỉ
để che regression.

## Tiêu chí hoàn thành một task tách file

- module mới compile độc lập trong crate;
- public/private visibility không mở rộng ngoài cần thiết;
- behavior và error contract không đổi;
- test cũ liên quan vẫn pass;
- có test mới nếu boundary mới tạo ra invariant;
- `git diff` chỉ chứa đúng task đó;
- commit message mô tả đúng responsibility vừa tách.
