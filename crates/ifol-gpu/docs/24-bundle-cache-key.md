# IFOL GPU: bundle cache key

Render node bundle hiện có cache key deterministic gồm:

- color/depth format của target;
- pipeline handle và pipeline version;
- bind group handle/version/dynamic offsets;
- mesh handle/version.

Khi key thay đổi, bundle được compile lại dù dirty flag của node chưa đổi. Đây là
nền tảng invalidation; context surface, sample count/MSAA và resource usage đầy đủ
vẫn phải được bổ sung trước production cache.
