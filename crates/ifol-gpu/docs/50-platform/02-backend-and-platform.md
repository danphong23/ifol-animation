# IFOL GPU: Backend và chiến lược đa nền tảng

## Abstraction backend

Trong source, builder, engine và capability lấy từ adapter/device thuộc module
`src/backend/`. `src/api/` chỉ là lớp facade host và re-export các type này để
giữ tương thích. Không đưa graph semantic, window event loop hoặc domain engine
vào `backend`.

`wgpu` là lớp portability. `ifol-gpu` không được giả định một native backend, channel order, surface format hay cơ chế presentation cụ thể.

Builder phải cho phép chọn backend rõ ràng và thực sự sử dụng lựa chọn đó khi tạo `wgpu::Instance`. Một field cấu hình không được dùng trong quá trình tạo instance là lỗi correctness.

## Các lớp platform

Core device API phải chạy được headless. Window integration là tùy chọn:

- native surface adapter: do host hoặc integration `winit` tùy chọn cung cấp;
- web: canvas/WebGPU surface do host cung cấp;
- mobile: native surface/context do host cung cấp;
- offline/headless: không có surface, chỉ dùng offscreen target và readback.

`winit`, image encoding và window event loop không nên là dependency bắt buộc của core tối giản.

## Mô hình capability

Capability phải phân biệt:

- adapter identity và backend;
- optional feature được hỗ trợ;
- device limit;
- surface capability của một surface cụ thể;
- portability/downlevel restriction.

`supports_compute` không được chỉ suy ra từ một limit khác không bằng 0. Feature phải lấy từ feature set thực tế; required feature/limit phải cấu hình được qua builder.

## Quy tắc surface

Surface configuration là nguồn sự thật cho format, kích thước, present mode, alpha mode và usage. Renderer tuyệt đối không được mặc định `Bgra8UnormSrgb` hay một format cố định khác.

Việc acquire surface, xử lý resize/lost/outdated và present thuộc lớp platform. Graph executor có thể render vào surface view được truyền vào, nhưng không nên sở hữu vòng đời window của host.

## Contract đa nền tảng

Thư viện cần công bố feature tier thay vì tuyên bố mọi nơi có hành vi giống hệt nhau:

- baseline render tier;
- optional compute tier;
- optional indirect/timestamp/storage tier;
- surface/presentation tier.

Ứng dụng chọn tier và tự cung cấp fallback bên ngoài core khi capability không tồn tại.
