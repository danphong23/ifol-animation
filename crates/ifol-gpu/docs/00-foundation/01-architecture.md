# IFOL GPU: Kiến trúc

## Phân tầng

```text
Host/ứng dụng
    -> graph builder + resource manager
    -> public API của ifol-gpu
    -> graph validation/compiler
    -> wgpu device/queue
    -> native hoặc WebGPU backend
```

Crate không được phụ thuộc ECS hoặc crate ứng dụng. Host có thể build graph ở một thread và submit ở render thread, tùy theo ownership rule của API.

## Kiến trúc mục tiêu

```text
backend/
  instance, adapter, device, surface, capabilities
resource/
  texture, buffer, sampler, shader, pipeline, bind group, handles
graph/
  graph, pass, usage, dependency, compiler
command/
  render, compute, copy, present
memory/
  upload, frame, transient, readback
execution/
  frame context, submission, synchronization
validation/
  graph/resource/pipeline diagnostics
```

Các module hiện tại (`api`, `render`, `memory`) là tổ chức trung gian. Những thay đổi sau này nên tiến gần các ranh giới trên mà không đưa khái niệm cấp cao vào core.

## Trách nhiệm của từng đối tượng

- `GpuInstance` quản lý chọn backend và tìm adapter.
- `GpuDevice` quản lý việc tạo resource và giới hạn của device.
- `GpuQueue` quản lý upload và submission.
- Resource store quản lý GPU object và validate handle.
- Graph mô tả công việc và resource usage, không sở hữu application state.
- Compiler chuyển graph thành execution plan cho một device/context.
- Execution context quản lý frame state, transient allocation và submission tracking.
- Cache quản lý compiled artifact, với key bao gồm mọi thuộc tính ảnh hưởng đến tính hợp lệ.

## Những điều không phải invariant

Các mệnh đề sau chỉ là policy, không phải chân lý kiến trúc cố định:

- một graph bắt buộc bằng một render pass;
- một frame bắt buộc bằng một queue submission;
- mọi node đều phải có render bundle;
- mỗi draw bắt buộc là một node;
- mọi graph bắt buộc có dạng cây;
- state sorting lúc nào cũng có lợi.

Compiler có thể dùng các policy này khi phù hợp, nhưng public model không được khóa cứng để sau này không thể thay đổi.
