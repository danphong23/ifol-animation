# Release readiness và feature freeze

`ifol-ecs` được xem là sẵn sàng làm nền cho các crate tiếp theo khi các điều
kiện sau cùng xanh:

- `cargo test -p ifol-ecs --all-targets`;
- `cargo test -p ifol-ecs --doc`;
- `cargo check -p ifol-ecs --all-targets`;
- `cargo fmt --package ifol-ecs -- --check`;
- `cargo clippy -p ifol-ecs --all-targets -- -D warnings`.

## Semantics cần giữ ổn định

- `World::insert` tự đăng ký component type nếu chưa có.
- System access không được tự động mở rộng: `AccessDescriptor` phải khai báo
  component và được validate khi `compile()`.
- Mutable query từ core chỉ cung cấp các signature đã kiểm tra alias; trait
  `WorldQueryMut` là `unsafe` extension point dành cho code chứng minh được
  invariant của chính nó.
- Thay đổi phase graph làm schedule stale; `compile()` thất bại không giữ lại
  schedule executable cũ.
- `clear()` giữ registrations, còn `shutdown()` reset toàn bộ runtime.

## Bằng chứng kiểm thử

Acceptance slices là correctness tests. Example
`examples/comprehensive_test.rs` là diagnostic smoke suite và benchmark thủ
nghiệm; số đo throughput phụ thuộc máy, không phải release gate.

`cargo fmt --all -- --check` là workspace gate riêng. Nó chỉ được dùng để đóng
toàn repository sau khi các crate khác cũng đã được format; trạng thái đó không
được dùng để đánh giá riêng `ifol-ecs`.

Test reports trong `tests/reports/` là tài liệu diễn giải. Chúng không được
dùng để thay thế output của lệnh test hiện tại và không nên chứa claim hiệu
năng tuyệt đối.
