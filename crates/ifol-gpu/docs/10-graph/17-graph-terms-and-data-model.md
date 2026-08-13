# IFOL GPU Graph Engine: Thuật ngữ và data model

## 1. Graph

Container mô tả một đơn vị công việc GPU có input, output, node/pass và dependency.

Graph có thể được lồng để tổ chức code, nhưng sau compile sẽ được flatten thành execution plan.

## 2. Node

Node là đơn vị logical trong graph. Node có thể là pass hoặc subgraph.

```text
Node
├── NodeId
├── Label
├── Kind
├── Inputs
├── Outputs
├── Resource usages
├── Commands
└── Metadata
```

Node không nên chứa compiled `wgpu::RenderBundle` như source of truth.

## 3. Pass

Pass là node có execution behavior cụ thể.

```text
Pass
├── RenderPass
├── ComputePass
├── CopyPass
├── ResolvePass
└── PresentPass
```

### RenderPass

```text
RenderPass
├── Color attachments
├── Depth/stencil attachment
├── Load/store operations
├── Viewport/scissor
├── Render commands
└── Resource usages
```

### ComputePass

```text
ComputePass
├── Compute pipeline
├── Bind groups
├── Dispatch / indirect dispatch
├── Storage reads/writes
└── Resource usages
```

### CopyPass

```text
CopyPass
├── Source resource
├── Destination resource
├── Copy region
└── Required usage
```

## 4. Resource

Resource là dữ liệu GPU được pass đọc hoặc ghi.

```text
Resource
├── Texture / TextureView
├── Buffer
├── Sampler
├── BindGroup
├── Pipeline
└── External Surface
```

## 5. Resource usage

Usage mô tả cách pass dùng resource:

```text
Read
Write
ReadWrite
ColorAttachment
DepthAttachment
StorageRead
StorageWrite
VertexInput
IndexInput
UniformInput
CopySource
CopyDestination
Present
```

Usage dùng để validate và suy ra dependency.

## 6. Dependency

Dependency là quan hệ “A phải hoàn thành trước B”. Có hai loại:

```text
Resource dependency: B đọc dữ liệu A đã ghi
Explicit dependency: host yêu cầu A trước B
```

## 7. Command

Command là hành động nhỏ nhất được encode vào pass.

```text
RenderCommand
├── SetPipeline
├── SetBindGroup
├── SetVertexBuffer
├── SetIndexBuffer
├── Draw
├── DrawIndexed
└── DrawIndirect
```

Command không tự quyết định dependency; pass/resource declaration chịu trách nhiệm đó.

## 8. Execution plan

Execution plan là graph đã validate và flatten:

```text
ExecutionPlan
├── Ordered flat passes
├── Resolved resource handles
├── Resource lifetime intervals
├── Barriers/usage transitions nếu cần
├── Cache decisions
├── Submission groups
└── Debug/profiling map
```

## 9. Submission group

Một submission group là nhóm command buffer được submit cùng nhau. Graph không bắt buộc chỉ có một group.

## 10. Compiled artifact

Compiled artifact là kết quả backend-specific như:

- render bundle;
- pipeline cache reference;
- encoded command template;
- resource binding plan.

Artifact phải có context key và version để tránh reuse sai.
