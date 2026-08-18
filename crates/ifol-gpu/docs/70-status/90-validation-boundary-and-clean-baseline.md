# Ranh giới chứng nhận và baseline sạch của `ifol-gpu`

Ngày cập nhật: 2026-08-18

## Baseline chính thức

Baseline kiểm thử đáng tin cậy gần nhất là commit `4d90857`, trong đó batch
TC68–TC70 đã được kiểm tra Desktop/Web và ghi nhận report tiếng Việt riêng.

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

- TC01–TC70 là phạm vi bằng chứng hiện tại; TC68–TC70 là batch đã commit gần
  nhất. Một số TC đạt raw parity tuyệt đối, một số TC đạt có điều kiện và report
  đã ghi rõ byte/pixel diff.
- TC71 chưa phải baseline: Web test từng cho kết quả nondeterministic ở invariant
  thứ tự bitonic sort. Đây trước hết là vấn đề của shader/graph test contract,
  chưa phải bằng chứng lỗi lõi `ifol-gpu`.
- TC72–TC73 có raw readback và numeric validation đạt trong lần thử hiện tại,
  nhưng preview/report chưa đủ sạch để chứng nhận batch; output preview Web có
  thể stale khi chạy focused mode với `skip_preview=1`.
- Không chạy mở rộng TC74+ cho đến khi baseline, report và ranh giới trách
  nhiệm giữa core/test harness/tầng media được chốt lại.

## Quy tắc commit và handoff

Mỗi commit chỉ chứa một batch coherent đã kiểm chứng. Không stage hoặc commit:

- test đang dang dở hoặc nondeterministic;
- PNG/bin/json output sinh tự động nếu không phải fixture chính thức;
- thay đổi ngoài `ifol-gpu` như `ifol-ecs`, `.agents` và `Cargo.lock`;
- diff lớn không rõ responsibility như thay đổi line ending của harness.

Trước task tiếp theo phải kiểm tra `git status`, giữ commit `4d90857` làm mốc
đối chiếu và chỉ nâng baseline sau khi test/report của batch mới hoàn tất.
