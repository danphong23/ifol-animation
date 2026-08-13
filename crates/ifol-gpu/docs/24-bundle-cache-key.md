# IFOL GPU: bundle cache key

Bundle key deterministic hiện bao gồm:

- context key của device/viewport;
- color/depth format và sample count;
- pipeline handle/version và layout metadata;
- bind-group handle/version/dynamic offsets;
- mesh handle/version.

Khi bất kỳ thành phần nào thay đổi, bundle được compile lại dù dirty flag của
node chưa đổi. Dynamic offset được đưa vào key để không bake dữ liệu frame cũ.

Bundle là optimization, không phải source of truth; segmented direct encode vẫn
là đường correctness. Context key do host cấp và host chịu trách nhiệm không
dùng chung bundle giữa context có lifetime khác nhau. Xem [context-aware cache](79-context-aware-bundle-cache.md).
