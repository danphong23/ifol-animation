# IFOL GPU: Phạm vi và những điều không làm

Trạng thái: nền tảng thiết kế, implementation hiện tại chưa hoàn chỉnh.

## Mục đích

`ifol-gpu` là thư viện GPU cấp thấp, không phụ thuộc domain. Thư viện nhận các GPU resource, shader/pipeline, command và execution graph rõ ràng; sau đó validate, compile và submit công việc thông qua `wgpu`.

Thư viện có thể được dùng cho game, animation, phim, compositing, xử lý video, offline rendering và các công cụ chuyên dụng. Thư viện không quyết định entity, scene, clip, layer, camera, material, asset hay timeline có ý nghĩa gì.

## Phạm vi thuộc `ifol-gpu`

- khởi tạo adapter/device/queue và báo cáo capability;
- tích hợp surface và presentation ở mức tùy chọn;
- quản lý ownership và lifetime của GPU resource;
- render graph, compute graph, copy graph, resolve và present;
- command recording và submission;
- cấp phát memory theo frame và transient resource;
- validation, diagnostics, profiling hook và readback;
- hành vi portable trên các backend native/WebGPU được `wgpu` hỗ trợ.

## Không thuộc phạm vi

Các phần sau thuộc tầng cao hơn:

- ECS, scene graph, camera, transform, animation, gameplay và physics;
- asset decoding, project file, video/audio codec và virtual filesystem;
- material, shader graph, node editor, UI, timeline và MCP command;
- chính sách fallback domain-specific, ví dụ thay ảnh bị thiếu;
- quyết định draw order, culling, transparency sorting hoặc LOD.

## Nguyên tắc thiết kế

1. Core phải rõ ràng: không tự ngầm tạo resource, đổi thứ tự hoặc fallback làm thay đổi semantics.
2. Core phải portable: platform/window integration là phần tùy chọn và được cô lập.
3. Core phải an toàn: stale handle, graph dependency sai, format không tương thích và reuse không an toàn phải được báo trước khi submit nếu có thể.
4. Core phải mở rộng được: render, compute và copy dùng chung mô hình resource/dependency.
5. Core ít policy: optimization policy phải cấu hình được và không tự đổi thứ tự công việc phụ thuộc thứ tự.

## Phân loại implementation hiện tại

Crate hiện tại là prototype của phần render. Nó có nhiều thử nghiệm hữu ích, nhưng public API chưa phải contract cuối cùng. Resource management, synchronization, graph dependency, surface format và bundle cache cần được thiết kế lại trước khi dùng production.
