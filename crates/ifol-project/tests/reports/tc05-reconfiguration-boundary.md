# TC05 — boundary reconfigure

Trạng thái: PASS

Đầu vào: composition đang chạy, sau đó thực hiện reconfigure rỗng.

Kỳ vọng: composition swap thành công, revision tăng sau step trước đó, runtime
vẫn ở `Ready`,
and the old package system is no longer executed.

Giới hạn đã biết: swap hiện tạo ECS runtime mới. Entity/component state cũ
không được bảo toàn. State transfer trong tương lai phải là contract tường minh,
không được suy diễn ngầm.
