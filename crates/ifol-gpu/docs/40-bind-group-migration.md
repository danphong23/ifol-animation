# Bind group mutation qua registry API

Các example, benchmark và test runtime đăng ký bind group bằng
`ResourceRegistry::insert_bind_group`. API này tăng bind-group version, giúp
bundle/cache biết resource backing đã thay đổi.

Không nên mutate `registry.bind_groups` trực tiếp; map public còn tồn tại chỉ để
giữ compatibility cho prototype và sẽ được private hóa sau khi migrate xong
các resource family.
