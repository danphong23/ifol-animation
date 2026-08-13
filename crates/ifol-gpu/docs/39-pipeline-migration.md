# Pipeline mutation qua registry API

Các example và benchmark đã chuyển mutation pipeline từ
`registry.pipelines.insert(...)` sang `registry.insert_pipeline(...)`. API mới
luôn tăng resource version, nên bundle/cache có thể nhận biết pipeline bị thay
thế.

`ultimate_test_suite.rs` vẫn còn mutation trực tiếp vì file đó đang có thay đổi
prototype riêng trong working tree; nó sẽ được migrate ở task riêng cùng phần
còn lại của raw public maps, không trộn vào commit này.
