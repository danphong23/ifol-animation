# Hướng dẫn: public API và extension boundary

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

