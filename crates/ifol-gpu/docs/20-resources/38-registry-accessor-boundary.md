# Registry accessor boundary

Compiler không còn truy cập trực tiếp các map resource của
`ResourceRegistry`. Mọi lookup/contains trong execution và validation đi qua
getter có kiểu (`pipeline`, `compute_pipeline`, `buffer`, `mesh`, `bind_group`,
`texture`) hoặc API `contains_*`.

Mutation mới nên dùng `insert_*`, `remove_*`, `mark_*_changed` để version cache
được cập nhật nhất quán. Các map public của prototype vẫn còn tạm thời để
examples cũ tương thích; chúng là debt cần đóng ở task migrate examples tiếp
theo. Việc tách compiler khỏi layout map là bước trung gian bắt buộc trước khi
đổi chúng thành private store.
