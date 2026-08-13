# IFOL GPU: Ma trận edge case

## Quy ước

- **Từ chối**: trả error, không submit.
- **Hỗ trợ**: có behavior được test.
- **Phụ thuộc capability**: chỉ chạy khi device/adapter hỗ trợ.
- **Không thuộc core**: xử lý ở tầng host.

| Nhóm | Edge case | Hành vi bắt buộc |
|---|---|---|
| Device | Không có adapter | Trả initialization error có context |
| Device | Adapter không có required feature | Trả unsupported-feature error |
| Device | Required limit quá cao | Trả limit error |
| Device | Backend bị cấm | Không tạo instance bằng backend đó |
| Surface | Surface format khác BGRA | Dùng format configuration, không hard-code |
| Surface | Width/height bằng 0 | Không configure hoặc trả resize error rõ ràng |
| Surface | Surface lost/outdated | Trả trạng thái để host recreate/reconfigure |
| Surface | Acquire timeout | Trả surface error, không panic |
| Resource | Handle stale | Từ chối lookup |
| Resource | Handle sai loại | Từ chối lookup |
| Resource | Destroy resource đang in-flight | Deferred destruction |
| Resource | Texture usage thiếu | Từ chối thao tác không hợp lệ |
| Resource | Texture descriptor zero size | Từ chối |
| Resource | Texture size overflow | Từ chối |
| Resource | Format attachment không khớp pipeline | Từ chối trước submit |
| Resource | Depth format không khớp pipeline | Từ chối trước submit |
| Binding | Slot âm/slot vượt limit | Từ chối, không index array cố định |
| Binding | Dynamic offset sai alignment | Từ chối |
| Binding | Dynamic offset vượt buffer | Từ chối |
| Binding | Bind group sai layout | Từ chối |
| Mesh | Mesh không tồn tại | Trả error, không silent skip |
| Mesh | Index range vượt buffer | Từ chối |
| Mesh | Vertex buffer thiếu | Từ chối |
| Graph | Graph rỗng | Hợp lệ nếu target/attachment hợp lệ |
| Graph | Node không tồn tại | Compile error |
| Graph | Self-cycle | Compile error |
| Graph | Cycle nhiều pass | Compile error |
| Graph | Read trước write | Compile error hoặc yêu cầu resource external rõ ràng |
| Graph | Hai write xung đột | Yêu cầu dependency/order rõ ràng |
| Graph | Subgraph output thiếu | Compile error |
| Graph | Subgraph dùng target khác format | Compile riêng theo context |
| Cache | Color format đổi | Cache miss/recompile |
| Cache | Depth/sample đổi | Cache miss/recompile |
| Cache | Pipeline version đổi | Invalidate artifact |
| Cache | Dynamic data đổi | Không dùng bundle chứa offset cũ |
| Memory | Allocation size 0 | Quy định rõ: no-op hoặc error; phải test |
| Memory | Allocation lớn hơn capacity | Trả allocation error |
| Memory | Ring wrap khi GPU chưa complete | Không wrap đè; chờ hoặc trả thiếu memory |
| Memory | Nhiều frame in-flight | Không corruption |
| Readback | Format không phải RGBA8 | Xử lý theo format hoặc từ chối rõ ràng |
| Readback | Bytes-per-row cần padding | Unpad đúng từng row |
| Readback | Map failure | Trả readback error |
| Execution | Submit nhiều lần | Hỗ trợ, không giả định một lần duy nhất |
| Execution | Device lost | Trả trạng thái recoverable/terminal rõ ràng |
| Execution | Missing pipeline | Error trước encode/submit |
| Execution | Panic từ invalid public input | Không được xảy ra |

## Quy tắc bổ sung

Mỗi edge case phải có ít nhất một trong:

- unit test;
- integration test;
- compile validation test;
- platform-specific test.

Không được chỉ ghi edge case trong tài liệu mà không có test owner.
