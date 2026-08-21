# TC01 — khởi tạo project

Trạng thái: PASS

Đầu vào: manifest project trong memory, yêu cầu một package.

Luồng: save/load `ProjectContainer`, chuyển thành `EngineConfig`, đăng ký
package candidate và build `EngineRuntime` headless.

Kỳ vọng: persistence của project nằm ngoài engine; runtime đạt `Ready`.
