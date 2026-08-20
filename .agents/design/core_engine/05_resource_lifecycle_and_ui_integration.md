# Service Lifecycle, Resource Ownership Và Output Integration

Tài liệu này định nghĩa cách package expose service instances cho feature systems
mà không làm engine/ECS kernel biết GPU, asset, codec hay UI.

---

## 1. Service Instance Có Scope `EngineRuntime`

Package render tạo hoặc nhận host binding cho `GpuService`, rồi đăng ký typed
resource provider để đặt handle lên `WORLD_ENTITY`. Engine chỉ thực thi provider
transaction; nó không import hoặc hard-code GPU. Đây là singleton theo phạm vi
runtime, không phải global mutable static. ECS không có resource storage riêng.

Asset, clock, input, network và subsystem tương lai dùng cùng contract. Provider
initialization explicit; không tự gọi `Default` hoặc discover global service.

Render Core có thể chia sẻ render-owned cache giữa nhiều request khi public GPU
contract và dependency revisions cho phép. Số lần submit, node reuse và bundle
cache không được hard-code tại tầng ECS/workspace.

---

## 2. Asset, Decode Và GPU Là Các Owner Khác Nhau

```text
Package-owned asset ID
→ package asset service resolve source/revision/artifact
→ Decode/Importer subsystem tạo representation khi feature yêu cầu
→ consuming package chọn dữ liệu cho step
→ Render Core/GpuService upload hoặc dùng GPU handle
```

Không chạy blocking file I/O/decode trong ECS tick. Background job trả handle/event
sẵn sàng; component chỉ giữ stable ID/handle/revision.

---

## 3. Chiến Lược Thu Dọn Rác (Eviction Strategy)

* ECS change tracking báo component/revision thay đổi.
* Content Feature quyết định contribution cần rebuild.
* Asset/Decode service quản lý cache của chính nó.
* `ifol-gpu` quản lý resource lifetime/deferred destruction theo submission
  contract của chính nó.

Xóa entity không đồng nghĩa GPU resource có thể destroy ngay. Không đặt ngưỡng
VRAM cố định cấp workspace khi backend chưa cung cấp telemetry/capability.

---

## 4. Output Không Phụ Thuộc UI

Render request có thể dùng surface, offscreen target hoặc readback. Platform
adapter sở hữu window/surface handle; Render Core và GpuService chỉ nhận boundary
đã chuẩn hóa.

### Hướng 1: Native/Web Surface
* Platform adapter tạo/resolve native hoặc web surface theo capability.
* `RenderRequestComponent` tham chiếu target identity, không giữ UI object.
* Không cam kết zero-latency hoặc zero-copy nếu chưa có runtime evidence.

### Hướng 2: Offscreen Texture (Preview / Embedded Panel)
*   `RenderRequestComponent` cài `output_target = RenderTarget::Offscreen { color: TextureHandle(P), .. }`.
*   GPU Engine vẽ kết quả ra `TextureHandle(P)`.
*   CLI/test/export có thể dùng readback; UI có thể dùng readback hoặc cơ chế
    present/shared resource nếu platform thực sự hỗ trợ.
