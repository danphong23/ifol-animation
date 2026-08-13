# Bind group mutation qua registry API

Các example, benchmark và test runtime đăng ký bind group bằng
`ResourceRegistry::insert_bind_group_with_descriptor`. API này validate
dynamic-offset contract và tăng bind-group version, giúp bundle/cache biết
resource backing đã thay đổi.

Không được mutate registry map trực tiếp; raw bind-group insertion đã bị loại
khỏi core để mọi resource đều đi qua descriptor contract.
