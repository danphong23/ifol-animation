# 6. Public API và extension boundary

Đây là phần cuối của [public usage guide](README.md). Extension chỉ cần đọc
sau khi đã nắm resource, graph và execution lifecycle.

## Contract operation và resource usage

Extension dùng như graph operation implement thêm `ExtensionOperation`. Nó phải
cung cấp danh sách `ResourceUsage` (resource, access, subresource) và tự kiểm
tra payload qua `validate_operation()`. Graph kernel chỉ dùng usage để dựng
dependency/hazard; không đọc semantic payload của extension.

## Public baseline cho engine bên ngoài

Engine ngoài chỉ nên dùng `GpuEngineBuilder`, `GpuCapabilities`, descriptor-based
resource registration, `RenderGraph`, command types, usage/dependency,
`RenderGraphExecutor::execute_checked`, typed errors, readback và lifecycle API.

Không phụ thuộc helper private của compiler hoặc raw implementation map,
`TextureCache` alias hoặc field nội bộ của registry.

## Built-in extension

Operation chung của GPU được thêm trong module `extensions/` và phải có
descriptor/config, resource usage declaration, validation, flat-plan
representation, encoder và unit/runtime test.

## Custom operation

Custom graph operation được đăng ký qua `ExtensionDispatchRegistry`. Host có thể
thêm operation mới mà không làm graph kernel biết semantic domain, đồng thời vẫn
giữ validation và resource lifetime contract.
### Giai đoạn đăng ký hiện tại

`ifol_gpu::extensions::ExtensionRegistry` cung cấp ranh giới đăng ký không phụ
thuộc domain. Mỗi extension có `ExtensionId` không rỗng, `version` và
implementation `GpuExtension`; registry từ chối ID trùng.

`RenderNode::Extension` giữ `ExtensionId` và `ResourceUsage`, được flatten như các
node khác. Dispatcher nhận `ExtensionExecutionContext` để encode command vào
`wgpu::CommandEncoder`; payload semantic vẫn nằm trong implementation của host.

Tạo executor có dispatcher bằng `RenderGraphExecutor::with_extension_dispatchers`.
Executor mặc định không có dispatcher và sẽ fail-closed bằng
`UnsupportedExtension`.
## Execution contract

Khi operation đã được đưa vào graph nhưng chưa có executor dispatch, việc
execute phải trả lỗi typed `UnsupportedExtension`; core không được bỏ qua node
hoặc âm thầm tiếp tục.
