# IFOL GPU: bundle cache và sample context

`RenderBundleEncoder` được tạo với `sample_count`. Đây là một phần của
render-pass compatibility, không chỉ là tối ưu hiệu năng.

Cache key của bundle hiện bao gồm:

- color format;
- depth/stencil format;
- sample count;
- pipeline handle/version;
- bind-group handle/version/dynamic offsets;
- mesh handle/version.

Vì vậy một node không được tái sử dụng bundle single-sample cho target MSAA
hoặc ngược lại. Khi sample count thay đổi, key đổi và bundle được compile lại.

Đây mới là một phần của context-aware cache. Device identity, multiview,
pipeline-layout compatibility và các capability-specific format rules vẫn phải
được bổ sung trước khi dùng chung một `RenderNodePool` giữa nhiều device/context.

## Test gate

Test unit chứng minh key đổi khi pipeline version đổi và khi sample count đổi;
MSAA execution tests kiểm chứng bundle được encode theo sample count của target.
