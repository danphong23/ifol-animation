# IFOL GPU Graph Engine: Tổng quan và phạm vi sử dụng

## Kết luận thiết kế

`ifol-gpu` không nên được hiểu là thư viện chỉ để vẽ ảnh. Nó là một **GPU work graph engine**: nhận một đồ thị các công việc GPU, resource và dependency; sau đó validate, flatten, compile và execute đồ thị đó trên `wgpu`.

Render là một workload quan trọng, nhưng không phải workload duy nhất.

## Graph có thể được dùng cho những gì?

### 1. Render 2D

- sprite, image, text quad;
- layer/compositing;
- alpha blend;
- mask và clipping;
- post-processing;
- canvas và UI rendering.

### 2. Render 2.5D

- camera orthographic/perspective;
- parallax layer;
- depth ordering;
- particle và effect;
- sprite trong không gian 3D;
- pre-compose nhiều lớp.

### 3. Render 3D

- forward rendering;
- deferred rendering;
- depth pre-pass;
- shadow map;
- reflection/refraction;
- G-buffer;
- lighting và post-process;
- multi-camera/multi-viewport.

### 4. Tính toán GPU

- particle simulation;
- physics/custom simulation;
- skinning và morph target;
- animation evaluation;
- GPU culling;
- sorting và compaction;
- prefix sum/reduction/scan;
- image processing;
- denoise, blur, resize, color conversion;
- neural/network inference nếu shader/model phù hợp.

### 5. Xử lý dữ liệu GPU

- upload CPU → GPU;
- copy buffer/texture;
- resolve MSAA;
- generate mipmap;
- readback GPU → CPU;
- staging và streaming;
- chuyển đổi format;
- chuẩn bị indirect draw command.

### 6. Offline rendering và media

- render frame đơn;
- render animation sequence;
- compositing video frame;
- preview frame;
- baking texture/lightmap;
- export image hoặc buffer cho tầng encode bên ngoài.

### 7. Công cụ và mô phỏng

- visual editor preview;
- shader playground;
- data visualization;
- scientific simulation;
- procedural generation;
- GPU benchmark và capability test.

## Core không biết domain nào

Graph engine chỉ biết các khái niệm thấp:

```text
Resource + Pass + Command + Usage + Dependency + Execution
```

Nó không biết:

```text
Entity, Sprite, Camera, VideoClip, Timeline, Material, ParticleSystem
```

Các domain bên ngoài dịch dữ liệu của mình thành graph/pass/resource.

## Mục tiêu của graph engine

```text
Graph dễ viết ở phía host
        ↓
Graph dễ validate
        ↓
Graph dễ flatten
        ↓
Execution plan phẳng, rõ ràng
        ↓
GPU command encoder/submission
```

Graph input có thể có cấu trúc lồng nhau để dễ tổ chức. Graph execution không cần giữ nguyên cấu trúc đó.
