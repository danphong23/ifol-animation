# Mesh mutation qua registry API

Mesh registration dùng `ResourceRegistry::insert_mesh_with_descriptor`, API này
validate metadata vertex/index và tăng mesh version để bundle/cache invalidation
hoạt động nhất quán. Mesh vẫn chứa vertex buffer, index buffer tùy chọn và
default count như contract hiện tại.

Raw mesh insertion đã bị loại khỏi core; mọi resource family phải đi qua cùng
nguyên tắc descriptor/replace/remove/version.
