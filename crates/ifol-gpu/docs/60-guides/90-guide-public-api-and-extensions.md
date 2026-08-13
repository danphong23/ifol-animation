# Hướng dẫn: public API và extension boundary

## Contract operation và resource usage

Extension dùng như graph operation implement thêm `ExtensionOperation`. Nó phải
cung cấp danh sách `ResourceUsage` (resource, access, subresource) và tự kiểm
tra payload qua `validate_operation()`. Graph kernel chỉ dùng usage để dựng
dependency/hazard; không đọc semantic payload của extension.

## Public baseline cho engine bên ngoài

Engine ngoài chỉ nên dùng `GpuEngineBuilder`, `GpuCapabilities`, descriptor-based
resource registration, `RenderGraph`, command types, usage/dependency,
`RenderGraphExecutor::execute_checked`, typed errors, readback và lifecycle API.

Không phụ thuộc helper private của compiler, raw compatibility insert API,
`TextureCache` alias hoặc field nội bộ của registry.

## Built-in extension

Operation chung của GPU được thêm trong module `extensions/` và phải có
descriptor/config, resource usage declaration, validation, flat-plan
representation, encoder và unit/runtime test.

## Custom operation

Custom graph operation là phần còn phải hoàn thiện. Mục tiêu là host có thể thêm
operation mới mà không làm graph kernel biết semantic domain, đồng thời vẫn giữ
validation và resource lifetime contract.
### Giai đoạn đăng ký hiện tại

`ifol_gpu::extensions::ExtensionRegistry` cung cấp ranh giới đăng ký không phụ
thuộc domain. Mỗi extension có `ExtensionId` không rỗng, `version` và
implementation `GpuExtension`; registry từ chối ID trùng.

Đây chưa phải execution contract: extension chưa được gắn vào `RenderNode` hay
flat plan. Task kế tiếp sẽ thêm operation payload, usage declaration, validation
và dispatch qua executor.
