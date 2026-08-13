# IFOL GPU: Testing và ma trận nền tảng

## Các tầng test

1. Unit test thuần: handle, descriptor key, alignment, range validation, graph topology và error path.
2. Device integration test: adapter/device creation, pipeline creation, buffer/texture usage và submission.
3. Render correctness test: offscreen image deterministic với tolerance và format assumption rõ ràng.
4. Cross-backend test: chạy cùng case trên Vulkan, Metal, DX12, GLES/WebGPU compatibility nếu được hỗ trợ.
5. Performance benchmark: đo riêng CPU graph build, compile/cache, encoding, submission và GPU completion.

## Quy tắc visual test

Snapshot hình ảnh hữu ích nhưng PNG không phải test đầy đủ. Mỗi snapshot case phải ghi:

- mô tả graph/pass;
- target format và kích thước;
- adapter/backend;
- expected image hoặc tolerance;
- test exact hay approximate;
- failure artifact và diagnostic trace.

Test harness không được âm thầm ghi đè golden image trong lần chạy test thông thường.

## Ma trận nền tảng

Dự án nên duy trì capability matrix thay vì tuyên bố mọi feature giống nhau ở mọi nơi:

| Nền tảng | Backend thường gặp | Surface | Headless | Compute | Ghi chú |
|---|---|---:|---:|---:|---|
| Windows | DX12/Vulkan/GLES | có | có | tùy capability | kiểm tra format/present |
| macOS | Metal | có | có | tùy capability | kiểm tra surface/storage limit |
| Linux | Vulkan/GLES | có | có | tùy capability | driver có thể khác nhau |
| Web | WebGPU/GLES compatibility | canvas | hạn chế | tùy capability | browser limit và async behavior |
| Android | Vulkan/GLES | có | có | tùy capability | chú ý lifecycle/surface loss |
| iOS | Metal | có | có | tùy capability | chú ý resource/surface lifecycle |

Ma trận là mục tiêu test, không phải lời hứa mọi feature có trên mọi nền tảng.

## Trạng thái hiện tại

Crate hiện có unit test cơ bản và visual example. Full example compilation hiện chưa sạch sau khi texture registry đổi API. Cần sửa việc này trước khi coi visual result là test authoritative.
