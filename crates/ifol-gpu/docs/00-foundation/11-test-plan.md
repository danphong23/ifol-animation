# IFOL GPU: Kế hoạch test bắt buộc

## Mục tiêu

Mọi task nâng cấp core phải hoàn thành test tương ứng trước khi chuyển sang task tiếp theo. Test được chia thành correctness, safety, portability và performance; không trộn các mục tiêu này.

## Lệnh kiểm tra chuẩn

Các lệnh mục tiêu của crate:

```text
cargo fmt --check
cargo check -p ifol-gpu --lib
cargo test -p ifol-gpu --lib
cargo test -p ifol-gpu --test check
cargo test -p ifol-gpu --tests
cargo test -p ifol-gpu --examples
cargo bench -p ifol-gpu --bench render_benchmarks
```

Trong giai đoạn baseline, nếu một lệnh chưa thể chạy do GPU/platform, phải có test skip có lý do rõ ràng; không được giả vờ pass.

## Unit test

### Handle và ID

- handle cùng giá trị thì equal/hash giống nhau;
- khác generation thì không resolve cùng resource;
- handle sau destroy bị từ chối;
- handle sai loại bị từ chối;
- ID tăng không overflow âm thầm;
- pool không trả về node đã destroy.

### Descriptor

- descriptor khác width/height/format/usage/sample/mip không bị xem là cùng resource;
- descriptor có zero dimension bị từ chối;
- size overflow bị từ chối;
- descriptor texture và target khai báo không khớp bị báo lỗi.

### Alignment và range

- request size bằng 0;
- request nhỏ hơn alignment;
- request đúng alignment;
- request lớn hơn capacity;
- offset không aligned;
- index range rỗng;
- index range vượt mesh;
- instance range rỗng và nhiều instance.

### Graph topology

- graph rỗng;
- một pass;
- nhiều pass tuyến tính;
- dependency fan-in/fan-out;
- self-cycle;
- cycle nhiều node;
- resource đọc trước khi ghi;
- resource ghi hai lần không có ordering;
- subgraph lồng nhiều tầng;
- subgraph output thiếu;
- node ID không tồn tại.

## Integration test

- headless device initialization;
- backend selection được áp dụng;
- required feature được chấp nhận hoặc trả lỗi;
- required limit vượt capability trả lỗi;
- tạo/destroy texture, buffer, sampler, bind group, pipeline;
- render offscreen clear;
- render indexed mesh;
- render procedural draw;
- depth test và depth clear;
- alpha blend;
- nhiều pass trong một command encoder;
- nhiều submission trong một frame;
- surface resize;
- surface format không mặc định;
- readback texture đúng format/aspect;
- shutdown khi còn resource in-flight.

## Cache và synchronization test

- cache miss tạo artifact;
- cache hit dùng lại artifact hợp lệ;
- đổi color format tạo cache miss;
- đổi depth format tạo cache miss;
- đổi sample count tạo cache miss;
- đổi pipeline version invalidate artifact;
- đổi bind layout invalidate artifact;
- dynamic offset frame sau không dùng offset frame trước;
- ring buffer không overwrite allocation in-flight;
- transient resource không reuse khi chưa complete;
- resource destroy được trì hoãn tới completion.

## Visual correctness test

- clear color;
- indexed triangle/quad;
- procedural fullscreen triangle;
- alpha blend;
- depth ordering;
- stencil/mask khi được hỗ trợ;
- texture sampling;
- offscreen compositing;
- nested subgraph;
- multiple color target khi được hỗ trợ;
- MSAA resolve khi được hỗ trợ;
- resize giữ đúng aspect/viewport.

Visual test phải có golden image, tolerance, format và backend metadata.

## Test gate

Một phase chỉ được coi là hoàn thành khi:

1. test mới của phase pass;
2. toàn bộ regression test pass;
3. không có panic/silent skip trong đường public tương ứng;
4. tài liệu implementation status được cập nhật;
5. task tiếp theo không phụ thuộc vào behavior chưa được test.
