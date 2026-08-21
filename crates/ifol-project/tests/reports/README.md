# Báo cáo kiểm thử tích hợp project và ifol-engine

Các báo cáo này mô tả bộ acceptance test thực thi tại
`../project_acceptance.rs`. Bộ test kiểm tra boundary thật của host:

```text
project storage -> ProjectContainer -> EngineConfig -> EngineBuilder -> EngineRuntime
```

Chạy bằng lệnh:

```text
cargo test -p ifol-project --test project_acceptance
```

Report là bằng chứng đọc được, không thay thế test thực thi. Mỗi report ghi
contract, lệnh chạy, kết quả quan sát và giới hạn đã biết. TC05 ghi rõ
reconfigure hiện thay bằng ECS runtime mới; đây chưa phải API migration entity
đang chạy.
