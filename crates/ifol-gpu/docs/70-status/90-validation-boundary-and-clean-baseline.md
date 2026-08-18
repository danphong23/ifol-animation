# Ranh giới chứng nhận và baseline sạch của `ifol-gpu`

Ngày cập nhật: 2026-08-18

## Baseline chính thức

Baseline kiểm thử đáng tin cậy gần nhất là commit `f41845e`. Batch TC101–TC105
đã được kiểm tra Desktop/Web theo canonical offscreen raw readback và có report
tổng hợp tiếng Việt tại [tc101-tc105-cross-platform-summary.md](../../tests/reports/tc101-tc105-cross-platform-summary.md).

Không có thay đổi production source trong batch handoff này. Các thay đổi đang
có trong working tree chủ yếu thuộc test runner, shader test, output và report;
không được xem là thay đổi đã chứng nhận của lõi `ifol-gpu`.

## Cách phân loại kết quả

- `ĐẠT`: test đúng mô tả, graph hợp lệ, validation không lỗi, state cold/warm
  ổn định và output đạt yêu cầu parity đã khai báo.
- `ĐẠT CÓ ĐIỀU KIỆN`: logic GPU, graph và cấu trúc output đúng; raw pixel còn
  khác do backend, floating-point, decoder hoặc format/presentation nằm ngoài
  lõi. Trạng thái này không chứng minh byte parity cho canonical media export.
- `CHƯA ĐẠT`: lỗi thuộc core, graph/shader contract hoặc test còn thiếu bằng
  chứng bắt buộc; ví dụ output không ổn định, numeric invariant sai,
  validation/resource hazard lỗi hoặc graph không cùng fingerprint.

Không được dùng sai khác do decoder, canvas, PNG/JPEG encoder, browser hoặc
driver để kết luận lõi `ifol-gpu` bị lỗi. Ngược lại, nếu scheduler, resource
hazard, binding, format contract hoặc cache behavior của core sai thì phải ghi
nhận là lỗi của `ifol-gpu`.

## Trạng thái test hiện tại

- TC01–TC105 đã có test/report trong repository; baseline gần nhất bổ sung
  batch TC101–TC105. TC101–TC104 đạt canonical parity gần như tuyệt đối.
- TC71–TC73 đã được rà lại: TC71 lệch 15 pixel với max delta 2; TC72 và TC73
  exact trên ảnh canonical. Không còn lý do giữ trạng thái pending cũ.
- TC105 đạt functional/vision parity với max delta 5/255 nhưng chưa byte-exact
  vì feedback sampling giữa backend; đây là giới hạn được ghi rõ, không phải
  lỗi canvas/gamma như kết quả cũ.
- Khi đánh giá test mới, phải dùng raw offscreen canonical; preview canvas chỉ
  để vision và không được dùng thay cho source of truth.

## Quy tắc commit và handoff

Mỗi commit chỉ chứa một batch coherent đã kiểm chứng. Không stage hoặc commit:

- test đang dang dở hoặc nondeterministic;
- PNG/bin/json output sinh tự động nếu không phải fixture chính thức;
- thay đổi ngoài `ifol-gpu` như `ifol-ecs`, `.agents` và `Cargo.lock`;
- diff lớn không rõ responsibility như thay đổi line ending của harness.

Trước task tiếp theo phải kiểm tra `git status`, dùng commit `f41845e` làm mốc
đối chiếu và chỉ nâng baseline sau khi test/report của batch mới hoàn tất.
