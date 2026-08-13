# Mesh mutation qua registry API

Mesh registration dùng `ResourceRegistry::insert_mesh`, API này tăng mesh
version để bundle/cache invalidation hoạt động nhất quán. Mesh vẫn chứa vertex
buffer, index buffer tùy chọn và default count như contract hiện tại.

Đây là bước migration trước khi đóng `meshes` map public; các resource family
còn lại phải đi qua cùng nguyên tắc insert/replace/remove/version.
